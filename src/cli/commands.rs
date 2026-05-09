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
    use crate::stealth::decoy::DecoyStreamGenerator;
    use crate::stealth::session_rotation::SessionRotator;
    use crate::stealth::traffic_shaping::TrafficShapingEngine;
    use crate::transport::connection::TransportEngine;
    use crate::transport::mode::ModeNegotiator;
    use crate::transport::quic_mask::{QuicMaskConnectPolicy, QuicTransport};
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

    let mut quic = QuicTransport::new(QuicMaskConnectPolicy::Success);
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

    let traffic_profile_name = crate::config::profiles::builtin_profile(&config.stealth.profile)
        .and_then(|p| p.traffic_profile)
        .unwrap_or_else(|| String::from("chrome_h3"));
    let mut shaping_engine = TrafficShapingEngine::from_profile_name(&traffic_profile_name);
    let mut decoy_gen = DecoyStreamGenerator::new(true);
    let mut session_rotator = SessionRotator::new(900, 2700);
    session_rotator.start(runtime.timer_wheel.now_ms());

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
            let shaped = shaping_engine.shape_packet(packet.as_bytes());
            let _ = engine.send_payload(shaped.payload);
            decoy_gen.on_packet_sent();
        }

        if let Some(chaff) = shaping_engine.should_send_chaff() {
            let _ = engine.send_payload(chaff.payload);
        }

        if let Some(decoy) = decoy_gen.should_inject_decoy() {
            let _ = engine.send_payload(decoy.data);
        }

        if session_rotator.should_rotate(runtime.timer_wheel.now_ms()) {
            println!(
                "{}",
                format_event(
                    EventCode::RuntimeReady,
                    &format!(
                        "session-rotate count={}",
                        session_rotator.rotation_count() + 1,
                    ),
                ),
            );
            let _ = engine.rekey();
            session_rotator.on_rotated(runtime.timer_wheel.now_ms(), 900, 2700);
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
    use crate::util::{AuthMethod, TransportMode};
    use std::net::SocketAddr;

    let config = load_config(args)?;

    if config.transport.mode == TransportMode::QuicMask {
        return run_server_quic(args);
    }
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

#[cfg(unix)]
fn create_udp_listener(addr: std::net::SocketAddr) -> Result<i32, String> {
    let domain = if addr.is_ipv4() {
        libc::AF_INET
    } else {
        libc::AF_INET6
    };

    let fd = unsafe { libc::socket(domain, libc::SOCK_DGRAM, 0) };
    if fd < 0 {
        return Err(String::from("UDP socket creation failed"));
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
        return Err(format!("UDP bind to {} failed", addr));
    }

    unsafe {
        libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK);
    }

    Ok(fd)
}

#[cfg(not(unix))]
fn create_udp_listener(_addr: std::net::SocketAddr) -> Result<i32, String> {
    Err(String::from(
        "UDP server listener not supported on this platform",
    ))
}

fn build_server_quic_config()
-> Result<(quinn_proto::ServerConfig, quinn_proto::ClientConfig), String> {
    use quinn_proto::crypto::rustls::{QuicClientConfig, QuicServerConfig};
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

    let cert = rcgen::generate_simple_self_signed(vec![String::from("localhost")])
        .map_err(|e| format!("cert generation failed: {e}"))?;
    let key = PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());
    let cert_der: CertificateDer = cert.cert.into();

    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], PrivateKeyDer::Pkcs8(key))
        .map_err(|e| format!("server TLS config failed: {e}"))?;
    server_crypto.alpn_protocols = vec![b"apate".to_vec()];

    let server_config = quinn_proto::ServerConfig::with_crypto(std::sync::Arc::new(
        QuicServerConfig::try_from(server_crypto)
            .map_err(|e| format!("QUIC server config failed: {e}"))?,
    ));

    let crypto_provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());
    let client_crypto = rustls::ClientConfig::builder_with_provider(crypto_provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("client TLS config failed: {e}"))?
        .dangerous()
        .with_custom_certificate_verifier(std::sync::Arc::new(
            crate::transport::quic_mask::SkipServerVerification,
        ))
        .with_no_client_auth();

    let client_config = quinn_proto::ClientConfig::new(std::sync::Arc::new(
        QuicClientConfig::try_from(client_crypto)
            .map_err(|e| format!("QUIC client config failed: {e}"))?,
    ));

    Ok((server_config, client_config))
}

fn run_server_quic(args: &CliArgs) -> Result<(), String> {
    use crate::runtime::Runtime;
    use crate::runtime::backend::FdInterest;
    use crate::stealth::session_rotation::SessionRotator;
    use quinn_proto::{DatagramEvent, Endpoint, EndpointConfig, Event, StreamEvent};
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Instant;

    let config = load_config(args)?;

    let mut runtime = Runtime::new();
    runtime.start().map_err(|e| e.to_string())?;

    let listen_addr: SocketAddr = config
        .server
        .listen
        .parse()
        .map_err(|e| format!("invalid server.listen: {e}"))?;

    let udp_fd = create_udp_listener(listen_addr)?;

    runtime
        .register_fd(
            1,
            udp_fd,
            FdInterest {
                readable: true,
                writable: false,
            },
        )
        .map_err(|e| e.to_string())?;

    let (server_config, _client_config) = build_server_quic_config()?;
    let endpoint_config = EndpointConfig::default();
    let mut endpoint = Endpoint::new(
        Arc::new(endpoint_config),
        Some(Arc::new(server_config)),
        true,
        None,
    );

    let mut cert_rotator = SessionRotator::new(3600, 7200);
    cert_rotator.start(runtime.timer_wheel.now_ms());

    println!(
        "{}",
        format_event(
            EventCode::Startup,
            &format!(
                "mode=server-quic listen={} backend={}",
                config.server.listen,
                runtime.backend_name(),
            ),
        ),
    );

    let mut connections: Vec<(quinn_proto::ConnectionHandle, quinn_proto::Connection)> = Vec::new();

    loop {
        runtime.tick().map_err(|e| e.to_string())?;

        #[cfg(unix)]
        {
            let mut buf = [0u8; 65536];
            let mut remote_storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
            let mut remote_len: libc::socklen_t =
                std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

            let n = unsafe {
                libc::recvfrom(
                    udp_fd,
                    buf.as_mut_ptr().cast(),
                    buf.len(),
                    0,
                    (&mut remote_storage as *mut libc::sockaddr_storage).cast(),
                    &mut remote_len,
                )
            };

            if n > 0 {
                let remote = sockaddr_to_socketaddr(&remote_storage);
                let now = Instant::now();
                let mut response_buf = Vec::new();

                if let Some(event) = endpoint.handle(
                    now,
                    remote,
                    None,
                    None,
                    bytes::BytesMut::from(&buf[..n as usize]),
                    &mut response_buf,
                ) {
                    match event {
                        DatagramEvent::NewConnection(incoming) => {
                            match endpoint.accept(incoming, now, &mut response_buf, None) {
                                Ok((handle, conn)) => {
                                    println!(
                                        "{}",
                                        format_event(
                                            EventCode::RuntimeReady,
                                            &format!("quic-accept handle={}", handle.0),
                                        ),
                                    );
                                    connections.push((handle, conn));
                                }
                                Err(e) => {
                                    println!(
                                        "{}",
                                        format_event(
                                            EventCode::AuthRejected,
                                            &format!("quic-accept-error={e:?}"),
                                        ),
                                    );
                                }
                            }
                        }
                        DatagramEvent::ConnectionEvent(ch, ce) => {
                            if let Some((_, conn)) = connections.iter_mut().find(|(h, _)| *h == ch)
                            {
                                conn.handle_event(ce);
                            }
                        }
                        DatagramEvent::Response(transmit) => {
                            send_udp_to(
                                udp_fd,
                                &response_buf[..transmit.size],
                                transmit.destination,
                            );
                        }
                    }
                }
            }
        }

        for (handle, conn) in &mut connections {
            let now = Instant::now();

            if let Some(deadline) = conn.poll_timeout()
                && now >= deadline
            {
                conn.handle_timeout(now);
            }

            let mut send_buf = Vec::with_capacity(1500);
            while let Some(transmit) = conn.poll_transmit(now, 1, &mut send_buf) {
                #[cfg(unix)]
                send_udp_to(udp_fd, &send_buf[..transmit.size], transmit.destination);
                let _ = transmit;
                send_buf.clear();
            }

            while let Some(ep_event) = conn.poll_endpoint_events() {
                if let Some(conn_event) = endpoint.handle_event(*handle, ep_event) {
                    conn.handle_event(conn_event);
                }
            }

            while let Some(event) = conn.poll() {
                match event {
                    Event::Connected => {
                        println!(
                            "{}",
                            format_event(
                                EventCode::HandshakeSuccess,
                                &format!("quic-connected handle={}", handle.0),
                            ),
                        );
                    }
                    Event::Stream(StreamEvent::Readable { id }) => {
                        let mut recv = conn.recv_stream(id);
                        if let Ok(mut chunks) = recv.read(true) {
                            while let Ok(Some(chunk)) = chunks.next(4096) {
                                println!(
                                    "{}",
                                    format_event(
                                        EventCode::RuntimeReady,
                                        &format!(
                                            "quic-data handle={} stream={} len={}",
                                            handle.0,
                                            id.index(),
                                            chunk.bytes.len(),
                                        ),
                                    ),
                                );
                            }
                            let _ = chunks.finalize();
                        }
                    }
                    _ => {}
                }
            }
        }

        if cert_rotator.should_rotate(runtime.timer_wheel.now_ms()) {
            if let Ok((new_server_config, _)) = build_server_quic_config() {
                endpoint.set_server_config(Some(Arc::new(new_server_config)));
                println!(
                    "{}",
                    format_event(
                        EventCode::RuntimeReady,
                        &format!(
                            "cert-rotate count={}",
                            cert_rotator.rotation_count() + 1,
                        ),
                    ),
                );
            }
            cert_rotator.on_rotated(runtime.timer_wheel.now_ms(), 3600, 7200);
        }
    }
}

#[cfg(unix)]
fn send_udp_to(fd: i32, data: &[u8], dest: std::net::SocketAddr) {
    match dest {
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
                libc::sendto(
                    fd,
                    data.as_ptr().cast(),
                    data.len(),
                    0,
                    (&sa as *const libc::sockaddr_in).cast(),
                    std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
                );
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
                libc::sendto(
                    fd,
                    data.as_ptr().cast(),
                    data.len(),
                    0,
                    (&sa as *const libc::sockaddr_in6).cast(),
                    std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
                );
            }
        }
    }
}

#[cfg(unix)]
fn sockaddr_to_socketaddr(storage: &libc::sockaddr_storage) -> std::net::SocketAddr {
    match storage.ss_family as libc::c_int {
        libc::AF_INET => {
            let sa: &libc::sockaddr_in = unsafe { &*(storage as *const _ as *const _) };
            let ip = std::net::Ipv4Addr::from(u32::from_be(sa.sin_addr.s_addr));
            std::net::SocketAddr::V4(std::net::SocketAddrV4::new(ip, u16::from_be(sa.sin_port)))
        }
        _ => {
            let sa: &libc::sockaddr_in6 = unsafe { &*(storage as *const _ as *const _) };
            let ip = std::net::Ipv6Addr::from(sa.sin6_addr.s6_addr);
            std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
                ip,
                u16::from_be(sa.sin6_port),
                sa.sin6_flowinfo,
                sa.sin6_scope_id,
            ))
        }
    }
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
