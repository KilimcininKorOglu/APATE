use crate::cli::args::{CliArgs, Command};
use crate::config::parser::parse_config;
use crate::config::types::AppConfig;
use crate::telemetry::{EventCode, format_event};
use std::fs;

pub fn dispatch(args: &CliArgs) -> Result<(), String> {
    match args.command {
        Command::Client => run_client(args),
        Command::Server => run_server(args),
        Command::GenKey => run_gen_key(),
        Command::Version => run_version(),
        Command::Help => run_help(),
    }
}

fn load_config(args: &CliArgs) -> Result<AppConfig, String> {
    let config_path = args
        .config_path
        .as_deref()
        .unwrap_or("/etc/apate/apate.conf");

    let source = fs::read_to_string(config_path)
        .map_err(|e| format!("cannot read config {config_path}: {e}"))?;

    let config = parse_config(&source).map_err(|e| format!("config parse error: {e}"))?;

    config
        .validate()
        .map_err(|e| format!("config validation error: {e}"))?;

    Ok(config)
}

fn run_client(args: &CliArgs) -> Result<(), String> {
    use crate::noise::handshake::HandshakeMachine;
    use crate::runtime::Runtime;
    use crate::runtime::backend::FdInterest;
    use crate::transport::connection::TransportEngine;
    use crate::transport::mode::ModeNegotiator;
    use crate::transport::quic_mask::{QuicMaskConnectPolicy, QuicMaskTransport};
    use crate::transport::tcp_tls::{TcpConnectPolicy, TcpTlsTransport};
    use crate::transport::udp_tls::{UdpConnectPolicy, UdpTlsTransport};
    use crate::tunnel::TunnelAdapter;

    let config = load_config(args)?;

    let mut runtime = Runtime::new();
    runtime.start().map_err(|e| e.to_string())?;

    let negotiator = ModeNegotiator::new(
        config.transport.mode,
        config.transport.fallback_timeout_secs,
    );

    let mut udp = UdpTlsTransport::new(UdpConnectPolicy::Success);
    udp.set_endpoint(config.client.server.clone());

    let mut tcp = TcpTlsTransport::new(TcpConnectPolicy::Success);
    tcp.set_endpoint(config.client.server.clone());

    let mut quic = QuicMaskTransport::new(QuicMaskConnectPolicy::Success);
    quic.set_endpoint(config.client.server.clone());

    let mut engine = TransportEngine::new(negotiator, udp, tcp, quic);

    let server_static_public = [0u8; 32];
    let handshake = HandshakeMachine::new(server_static_public);
    engine.set_handshake(handshake);

    println!(
        "{}",
        format_event(
            EventCode::Startup,
            &format!(
                "mode=client server={} transport={} backend={}",
                config.client.server,
                config.transport.mode.as_str(),
                runtime.backend_name(),
            ),
        ),
    );

    engine.establish().map_err(|e| e.to_string())?;

    println!(
        "{}",
        format_event(
            EventCode::HandshakeSuccess,
            &format!(
                "transport={:?} endpoint={}",
                engine.active_kind(),
                engine.endpoint(),
            ),
        ),
    );

    #[cfg(target_os = "macos")]
    let mut tun = crate::tunnel::MacOsTunAdapter::new(String::from("utun7"));
    #[cfg(target_os = "linux")]
    let mut tun = crate::tunnel::LinuxTunAdapter::new(String::from("apate0"));
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let mut tun = crate::tunnel::LinuxTunAdapter::new(String::from("apate0"));

    tun.open().map_err(|e| e.to_string())?;
    tun.configure(1400).map_err(|e| e.to_string())?;

    println!(
        "{}",
        format_event(
            EventCode::RuntimeReady,
            &format!("tunnel={} mtu={}", tun.name(), tun.mtu()),
        ),
    );

    if let Some(tun_fd) = tun.raw_fd() {
        runtime
            .register_fd(
                10,
                tun_fd,
                FdInterest {
                    readable: true,
                    writable: false,
                },
            )
            .map_err(|e| e.to_string())?;
    }

    if let Some(transport_fd) = engine.active_raw_fd() {
        runtime
            .register_fd(
                20,
                transport_fd,
                FdInterest {
                    readable: true,
                    writable: true,
                },
            )
            .map_err(|e| e.to_string())?;
    }

    loop {
        runtime.tick().map_err(|e| e.to_string())?;

        if let Some(packet) = tun.read_packet().map_err(|e| e.to_string())? {
            let payload = packet.as_bytes().to_vec();
            let _ = engine.send_payload(payload);
        }

        if let Some(frame) = engine.recv_frame().map_err(|e| e.to_string())?
            && let Ok(packet) = crate::tunnel::TunnelPacket::parse(&frame.payload)
        {
            let _ = tun.write_packet(packet);
        }
    }
}

fn run_server(args: &CliArgs) -> Result<(), String> {
    use crate::auth::static_key::StaticKeyBackend;
    use crate::auth::{
        AuthCoordinator, AuthInput, ProbeGatePolicy, ProbeGateResult, evaluate_probe_gate,
    };
    use crate::runtime::Runtime;
    use crate::runtime::backend::FdInterest;
    use crate::stealth::facade::FacadeResponder;
    use crate::util::AuthMethod;
    use std::net::SocketAddr;

    let config = load_config(args)?;
    let methods: Vec<&str> = config.auth.methods.iter().map(|m| m.as_str()).collect();

    let mut runtime = Runtime::new();
    runtime.start().map_err(|e| e.to_string())?;

    let policy = ProbeGatePolicy {
        facade_on_auth_failure: config.stealth.facade_on_auth_failure,
    };
    let facade = FacadeResponder::new(String::from("nginx"));

    let mut coordinator = AuthCoordinator::new(config.auth.methods.clone());
    for method in &config.auth.methods {
        if *method == AuthMethod::StaticKey {
            coordinator.register_backend(
                AuthMethod::StaticKey,
                Box::new(StaticKeyBackend::new(Vec::new())),
            );
        }
    }

    let listen_addr: SocketAddr = config
        .server
        .listen
        .parse()
        .map_err(|e| format!("invalid server.listen: {e}"))?;

    let listener_fd = create_listener(listen_addr)?;

    runtime
        .register_fd(
            1,
            listener_fd,
            FdInterest {
                readable: true,
                writable: false,
            },
        )
        .map_err(|e| e.to_string())?;

    println!(
        "{}",
        format_event(
            EventCode::Startup,
            &format!(
                "mode=server listen={} auth=[{}] backend={} facade={}",
                config.server.listen,
                methods.join(","),
                runtime.backend_name(),
                policy.facade_on_auth_failure,
            ),
        ),
    );

    let mut next_token: u64 = 100;

    loop {
        runtime.tick().map_err(|e| e.to_string())?;

        while let Some(token) = runtime.executor.poll_ready_task() {
            if token == 1 {
                let client_fd = accept_connection(listener_fd);
                if let Some(fd) = client_fd {
                    let client_token = next_token;
                    next_token += 1;

                    let _ = runtime.register_fd(
                        client_token,
                        fd,
                        FdInterest {
                            readable: true,
                            writable: false,
                        },
                    );

                    println!(
                        "{}",
                        format_event(
                            EventCode::RuntimeReady,
                            &format!("token={client_token} fd={fd}"),
                        ),
                    );
                }
            } else {
                let mut read_buf = [0u8; 4096];
                let bytes_read = {
                    #[cfg(unix)]
                    {
                        unsafe {
                            libc::recv(
                                token as i32,
                                read_buf.as_mut_ptr().cast(),
                                read_buf.len(),
                                0,
                            )
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        0isize
                    }
                };

                let auth_result = if bytes_read > 0 {
                    let input = AuthInput {
                        method: config
                            .auth
                            .methods
                            .first()
                            .copied()
                            .unwrap_or(AuthMethod::StaticKey),
                        payload: read_buf[..bytes_read as usize].to_vec(),
                    };
                    coordinator.authenticate(input)
                } else {
                    Err(crate::auth::AuthError::EmptyPayload)
                };

                let gate_result = evaluate_probe_gate(auth_result, policy);

                match gate_result {
                    ProbeGateResult::AllowTunnel(identity) => {
                        println!(
                            "{}",
                            format_event(
                                EventCode::HandshakeSuccess,
                                &format!("token={token} subject={}", identity.subject),
                            ),
                        );
                    }
                    ProbeGateResult::ServeFacade => {
                        let response = facade.respond_for_probe("/");
                        let http_bytes = FacadeResponder::to_http_bytes(&response);
                        #[cfg(unix)]
                        unsafe {
                            libc::send(
                                token as i32,
                                http_bytes.as_ptr().cast(),
                                http_bytes.len(),
                                0,
                            );
                        }
                        println!(
                            "{}",
                            format_event(
                                EventCode::AuthRejected,
                                &format!(
                                    "token={token} action=facade status={}",
                                    response.status_code
                                ),
                            ),
                        );
                    }
                    ProbeGateResult::Reject => {
                        println!(
                            "{}",
                            format_event(
                                EventCode::AuthRejected,
                                &format!("token={token} action=reject"),
                            ),
                        );
                    }
                }

                let _ = runtime.deregister_fd(token);
                #[cfg(unix)]
                unsafe {
                    libc::close(token as i32);
                }
            }
        }
    }
}

#[cfg(unix)]
fn create_listener(addr: std::net::SocketAddr) -> Result<i32, String> {
    let domain = if addr.is_ipv4() {
        libc::AF_INET
    } else {
        libc::AF_INET6
    };

    let fd = unsafe { libc::socket(domain, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(String::from("socket creation failed"));
    }

    let enable: libc::c_int = 1;
    unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_REUSEADDR,
            (&enable as *const libc::c_int).cast(),
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );
    }

    let bind_result = match addr {
        std::net::SocketAddr::V4(v4) => {
            let sa = libc::sockaddr_in {
                sin_family: libc::AF_INET as libc::sa_family_t,
                sin_port: v4.port().to_be(),
                sin_addr: libc::in_addr {
                    s_addr: u32::from_ne_bytes(v4.ip().octets()),
                },
                sin_zero: [0; 8],
                #[cfg(any(target_os = "macos", target_os = "freebsd"))]
                sin_len: std::mem::size_of::<libc::sockaddr_in>() as u8,
            };
            unsafe {
                libc::bind(
                    fd,
                    (&sa as *const libc::sockaddr_in).cast(),
                    std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
                )
            }
        }
        std::net::SocketAddr::V6(v6) => {
            let sa = libc::sockaddr_in6 {
                sin6_family: libc::AF_INET6 as libc::sa_family_t,
                sin6_port: v6.port().to_be(),
                sin6_flowinfo: v6.flowinfo(),
                sin6_addr: libc::in6_addr {
                    s6_addr: v6.ip().octets(),
                },
                sin6_scope_id: v6.scope_id(),
                #[cfg(any(target_os = "macos", target_os = "freebsd"))]
                sin6_len: std::mem::size_of::<libc::sockaddr_in6>() as u8,
            };
            unsafe {
                libc::bind(
                    fd,
                    (&sa as *const libc::sockaddr_in6).cast(),
                    std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
                )
            }
        }
    };

    if bind_result < 0 {
        unsafe {
            libc::close(fd);
        }
        return Err(format!("bind to {} failed", addr));
    }

    let listen_result = unsafe { libc::listen(fd, 128) };
    if listen_result < 0 {
        unsafe {
            libc::close(fd);
        }
        return Err(String::from("listen failed"));
    }

    unsafe {
        libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK);
    }

    Ok(fd)
}

#[cfg(not(unix))]
fn create_listener(_addr: std::net::SocketAddr) -> Result<i32, String> {
    Err(String::from(
        "server listener not supported on this platform",
    ))
}

#[cfg(unix)]
fn accept_connection(listener_fd: i32) -> Option<i32> {
    let fd = unsafe { libc::accept(listener_fd, std::ptr::null_mut(), std::ptr::null_mut()) };
    if fd < 0 {
        return None;
    }
    unsafe {
        libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK);
    }
    Some(fd)
}

#[cfg(not(unix))]
fn accept_connection(_listener_fd: i32) -> Option<i32> {
    None
}

fn run_gen_key() -> Result<(), String> {
    use crate::crypto::kx::derive_public_key;
    use crate::crypto::rng::os_seed;

    let secret = os_seed();
    let public = derive_public_key(secret);
    let hex: String = public.iter().map(|b| format!("{b:02x}")).collect();
    println!("public_key={hex}");
    Ok(())
}

fn run_version() -> Result<(), String> {
    println!("apate {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}

fn run_help() -> Result<(), String> {
    println!(
        "\
apate - stealth VPN tunnel

USAGE:
    apate <COMMAND> [OPTIONS]

COMMANDS:
    client      Start in client mode
    server      Start in server mode
    gen-key     Generate a new X25519 keypair
    version     Print version
    help        Print this help

OPTIONS:
    -c, --config <PATH>    Config file path (default: /etc/apate/apate.conf)
    -v, --verbose          Enable verbose logging
    -h, --help             Print help
    -V, --version          Print version"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::dispatch;
    use crate::cli::args::{CliArgs, Command};

    #[test]
    fn dispatch_version_succeeds() {
        let args = CliArgs {
            command: Command::Version,
            config_path: None,
            verbose: false,
        };
        assert!(dispatch(&args).is_ok());
    }

    #[test]
    fn dispatch_help_succeeds() {
        let args = CliArgs {
            command: Command::Help,
            config_path: None,
            verbose: false,
        };
        assert!(dispatch(&args).is_ok());
    }

    #[test]
    fn dispatch_gen_key_succeeds() {
        let args = CliArgs {
            command: Command::GenKey,
            config_path: None,
            verbose: false,
        };
        assert!(dispatch(&args).is_ok());
    }
}
