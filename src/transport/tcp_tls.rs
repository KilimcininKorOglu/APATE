use crate::transport::frame::{FRAME_HEADER_LEN, decode_frame, encode_frame};
use crate::transport::mode::AttemptOutcome;
use crate::transport::{Frame, TransportError, TransportStrategy};
use core::time::Duration;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpConnectPolicy {
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpTlsTransport {
    connect_policy: TcpConnectPolicy,
    connected: bool,
    outbound: Vec<Frame>,
    inbound: VecDeque<Frame>,
    #[cfg(unix)]
    fd: Option<i32>,
    #[cfg(target_os = "windows")]
    fd: Option<usize>,
    endpoint: Option<String>,
}

impl TcpTlsTransport {
    pub fn new(connect_policy: TcpConnectPolicy) -> Self {
        Self {
            connect_policy,
            connected: false,
            outbound: Vec::new(),
            inbound: VecDeque::new(),
            #[cfg(any(unix, target_os = "windows"))]
            fd: None,
            endpoint: None,
        }
    }

    pub fn set_endpoint(&mut self, endpoint: String) {
        self.endpoint = Some(endpoint);
    }

    pub fn raw_fd(&self) -> Option<i32> {
        #[cfg(unix)]
        {
            self.fd
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    pub fn connect(&mut self, _timeout: Duration) -> Result<AttemptOutcome, TransportError> {
        if let Some(ref endpoint) = self.endpoint {
            return self.connect_real(endpoint.clone());
        }

        match self.connect_policy {
            TcpConnectPolicy::Success => {
                self.connected = true;
                Ok(AttemptOutcome::Connected)
            }
            TcpConnectPolicy::Failure => Ok(AttemptOutcome::Failed),
        }
    }

    #[cfg(unix)]
    fn connect_real(&mut self, endpoint: String) -> Result<AttemptOutcome, TransportError> {
        use std::net::SocketAddr;

        let addr: SocketAddr = endpoint.parse().map_err(|_| TransportError::NotConnected)?;

        let fd = unsafe {
            libc::socket(
                if addr.is_ipv4() {
                    libc::AF_INET
                } else {
                    libc::AF_INET6
                },
                libc::SOCK_STREAM,
                0,
            )
        };
        if fd < 0 {
            return Ok(AttemptOutcome::Failed);
        }

        unsafe {
            libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK);
        }

        let result = match addr {
            SocketAddr::V4(v4) => {
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
                    libc::connect(
                        fd,
                        (&sa as *const libc::sockaddr_in).cast(),
                        std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
                    )
                }
            }
            SocketAddr::V6(v6) => {
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
                    libc::connect(
                        fd,
                        (&sa as *const libc::sockaddr_in6).cast(),
                        std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
                    )
                }
            }
        };

        if result < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if errno != libc::EINPROGRESS {
                unsafe {
                    libc::close(fd);
                }
                return Ok(AttemptOutcome::Failed);
            }
        }

        self.fd = Some(fd);
        self.connected = true;
        Ok(AttemptOutcome::Connected)
    }

    #[cfg(target_os = "windows")]
    fn connect_real(&mut self, endpoint: String) -> Result<AttemptOutcome, TransportError> {
        use std::net::SocketAddr;
        use windows_sys::Win32::Networking::WinSock::{
            AF_INET, AF_INET6, FIONBIO, INVALID_SOCKET, SOCK_STREAM, SOCKADDR_IN, WSAEWOULDBLOCK,
            WSAGetLastError, WSAStartup, closesocket, connect, ioctlsocket, socket,
        };

        let addr: SocketAddr = endpoint.parse().map_err(|_| TransportError::NotConnected)?;

        let mut wsadata = unsafe { std::mem::zeroed() };
        if unsafe { WSAStartup(0x0202, &mut wsadata) } != 0 {
            return Ok(AttemptOutcome::Failed);
        }

        let family = if addr.is_ipv4() {
            AF_INET as i32
        } else {
            AF_INET6 as i32
        };
        let sock = unsafe { socket(family, SOCK_STREAM as i32, 0) };
        if sock == INVALID_SOCKET {
            return Ok(AttemptOutcome::Failed);
        }

        let mut nonblock: u32 = 1;
        unsafe {
            ioctlsocket(sock, FIONBIO, &mut nonblock);
        }

        let result = match addr {
            SocketAddr::V4(v4) => {
                let sa = SOCKADDR_IN {
                    sin_family: AF_INET,
                    sin_port: v4.port().to_be(),
                    sin_addr: windows_sys::Win32::Networking::WinSock::IN_ADDR {
                        S_un: windows_sys::Win32::Networking::WinSock::IN_ADDR_0 {
                            S_addr: u32::from_ne_bytes(v4.ip().octets()),
                        },
                    },
                    sin_zero: [0i8; 8],
                };
                unsafe {
                    connect(
                        sock,
                        (&sa as *const SOCKADDR_IN).cast(),
                        std::mem::size_of::<SOCKADDR_IN>() as i32,
                    )
                }
            }
            SocketAddr::V6(_) => {
                unsafe {
                    closesocket(sock);
                }
                return Ok(AttemptOutcome::Failed);
            }
        };

        if result != 0 {
            let err = unsafe { WSAGetLastError() };
            if err != WSAEWOULDBLOCK {
                unsafe {
                    closesocket(sock);
                }
                return Ok(AttemptOutcome::Failed);
            }
        }

        self.fd = Some(sock);
        self.connected = true;
        Ok(AttemptOutcome::Connected)
    }

    #[cfg(not(any(unix, target_os = "windows")))]
    fn connect_real(&mut self, _endpoint: String) -> Result<AttemptOutcome, TransportError> {
        Ok(AttemptOutcome::Failed)
    }

    pub fn queue_inbound(&mut self, frame: Frame) {
        self.inbound.push_back(frame);
    }
}

impl TransportStrategy for TcpTlsTransport {
    fn send(&mut self, frame: Frame) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        #[cfg(unix)]
        if let Some(fd) = self.fd {
            let encoded = encode_frame(&frame, 0).map_err(TransportError::Frame)?;
            let mut offset = 0;
            while offset < encoded.len() {
                let sent = unsafe {
                    libc::send(
                        fd,
                        encoded[offset..].as_ptr().cast(),
                        encoded.len() - offset,
                        0,
                    )
                };
                if sent <= 0 {
                    return Err(TransportError::NotConnected);
                }
                offset += sent as usize;
            }
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        if let Some(sock) = self.fd {
            let encoded = encode_frame(&frame, 0).map_err(TransportError::Frame)?;
            let mut offset = 0;
            while offset < encoded.len() {
                let sent = unsafe {
                    windows_sys::Win32::Networking::WinSock::send(
                        sock,
                        encoded[offset..].as_ptr(),
                        (encoded.len() - offset) as i32,
                        0,
                    )
                };
                if sent <= 0 {
                    return Err(TransportError::NotConnected);
                }
                offset += sent as usize;
            }
            return Ok(());
        }

        self.outbound.push(frame);
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Frame>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        #[cfg(unix)]
        if let Some(fd) = self.fd {
            let mut header_buf = [0u8; FRAME_HEADER_LEN];
            let received = unsafe {
                libc::recv(
                    fd,
                    header_buf.as_mut_ptr().cast(),
                    FRAME_HEADER_LEN,
                    libc::MSG_PEEK,
                )
            };
            if received < FRAME_HEADER_LEN as isize {
                return Ok(None);
            }

            let payload_len = u16::from_be_bytes([header_buf[2], header_buf[3]]) as usize;
            let total_len = FRAME_HEADER_LEN + payload_len;
            let mut buf = vec![0u8; total_len];
            let received = unsafe { libc::recv(fd, buf.as_mut_ptr().cast(), total_len, 0) };
            if received < total_len as isize {
                return Ok(None);
            }

            let decoded = decode_frame(&buf[..received as usize]).map_err(TransportError::Frame)?;
            return Ok(Some(decoded.frame));
        }

        #[cfg(target_os = "windows")]
        if let Some(sock) = self.fd {
            use windows_sys::Win32::Networking::WinSock::MSG_PEEK;

            let mut header_buf = [0u8; FRAME_HEADER_LEN];
            let received = unsafe {
                windows_sys::Win32::Networking::WinSock::recv(
                    sock,
                    header_buf.as_mut_ptr(),
                    FRAME_HEADER_LEN as i32,
                    MSG_PEEK,
                )
            };
            if received < FRAME_HEADER_LEN as i32 {
                return Ok(None);
            }

            let payload_len = u16::from_be_bytes([header_buf[2], header_buf[3]]) as usize;
            let total_len = FRAME_HEADER_LEN + payload_len;
            let mut buf = vec![0u8; total_len];
            let received = unsafe {
                windows_sys::Win32::Networking::WinSock::recv(
                    sock,
                    buf.as_mut_ptr(),
                    total_len as i32,
                    0,
                )
            };
            if received < total_len as i32 {
                return Ok(None);
            }

            let decoded = decode_frame(&buf[..received as usize]).map_err(TransportError::Frame)?;
            return Ok(Some(decoded.frame));
        }

        Ok(self.inbound.pop_front())
    }
}

impl Drop for TcpTlsTransport {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(fd) = self.fd {
            unsafe {
                libc::close(fd);
            }
        }
        #[cfg(target_os = "windows")]
        if let Some(sock) = self.fd {
            unsafe {
                windows_sys::Win32::Networking::WinSock::closesocket(sock);
            }
        }
    }
}
