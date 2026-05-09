use crate::stealth::quic_camouflage::QuicCamouflagePacket;
use crate::transport::mode::AttemptOutcome;
use crate::transport::{Frame, TransportError, TransportStrategy};
use core::time::Duration;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuicMaskConnectPolicy {
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuicMaskTransport {
    connect_policy: QuicMaskConnectPolicy,
    connected: bool,
    connection_id: u32,
    next_packet_number: u16,
    outbound: Vec<Vec<u8>>,
    inbound: VecDeque<Vec<u8>>,
    #[cfg(unix)]
    fd: Option<i32>,
    #[cfg(target_os = "windows")]
    fd: Option<usize>,
    endpoint: Option<String>,
}

impl QuicMaskTransport {
    pub fn new(connect_policy: QuicMaskConnectPolicy) -> Self {
        Self {
            connect_policy,
            connected: false,
            connection_id: 1,
            next_packet_number: 0,
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
            QuicMaskConnectPolicy::Success => {
                self.connected = true;
                Ok(AttemptOutcome::Connected)
            }
            QuicMaskConnectPolicy::Failure => Ok(AttemptOutcome::Failed),
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
                libc::SOCK_DGRAM,
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
            unsafe {
                libc::close(fd);
            }
            return Ok(AttemptOutcome::Failed);
        }

        self.fd = Some(fd);
        self.connected = true;
        Ok(AttemptOutcome::Connected)
    }

    #[cfg(target_os = "windows")]
    fn connect_real(&mut self, endpoint: String) -> Result<AttemptOutcome, TransportError> {
        use std::net::SocketAddr;
        use windows_sys::Win32::Networking::WinSock::{
            AF_INET, AF_INET6, FIONBIO, INVALID_SOCKET, SOCK_DGRAM, SOCKADDR_IN, WSAStartup,
            closesocket, connect, ioctlsocket, socket,
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
        let sock = unsafe { socket(family, SOCK_DGRAM as i32, 0) };
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
            unsafe {
                closesocket(sock);
            }
            return Ok(AttemptOutcome::Failed);
        }

        self.fd = Some(sock);
        self.connected = true;
        Ok(AttemptOutcome::Connected)
    }

    #[cfg(not(any(unix, target_os = "windows")))]
    fn connect_real(&mut self, _endpoint: String) -> Result<AttemptOutcome, TransportError> {
        Ok(AttemptOutcome::Failed)
    }

    pub fn queue_inbound(&mut self, frame: Frame) -> Result<(), TransportError> {
        let packet_number = u16::try_from(frame.sequence)
            .map_err(|_| TransportError::Frame(crate::transport::FrameError::Malformed))?;
        let packet = QuicCamouflagePacket {
            connection_id: self.connection_id,
            packet_number,
            payload: frame.payload,
        };
        let encoded = packet
            .encode_masked()
            .map_err(|_| TransportError::Frame(crate::transport::FrameError::Malformed))?;
        self.inbound.push_back(encoded);
        Ok(())
    }
}

impl TransportStrategy for QuicMaskTransport {
    fn send(&mut self, frame: Frame) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        let packet = QuicCamouflagePacket {
            connection_id: self.connection_id,
            packet_number: self.next_packet_number,
            payload: frame.payload,
        };
        self.next_packet_number = self.next_packet_number.wrapping_add(1);
        let encoded = packet
            .encode_masked()
            .map_err(|_| TransportError::Frame(crate::transport::FrameError::Malformed))?;

        #[cfg(unix)]
        if let Some(fd) = self.fd {
            let sent = unsafe { libc::send(fd, encoded.as_ptr().cast(), encoded.len(), 0) };
            if sent < 0 {
                return Err(TransportError::NotConnected);
            }
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        if let Some(sock) = self.fd {
            let sent = unsafe {
                windows_sys::Win32::Networking::WinSock::send(
                    sock,
                    encoded.as_ptr(),
                    encoded.len() as i32,
                    0,
                )
            };
            if sent < 0 {
                return Err(TransportError::NotConnected);
            }
            return Ok(());
        }

        self.outbound.push(encoded);
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Frame>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        #[cfg(unix)]
        if let Some(fd) = self.fd {
            let mut buf = [0u8; 65536];
            let received = unsafe { libc::recv(fd, buf.as_mut_ptr().cast(), buf.len(), 0) };
            if received <= 0 {
                return Ok(None);
            }
            let packet = QuicCamouflagePacket::decode_masked(&buf[..received as usize])
                .map_err(|_| TransportError::Frame(crate::transport::FrameError::Malformed))?;
            return Ok(Some(Frame {
                frame_type: crate::transport::FrameType::Data,
                sequence: u64::from(packet.packet_number),
                payload: packet.payload,
            }));
        }

        #[cfg(target_os = "windows")]
        if let Some(sock) = self.fd {
            let mut buf = [0u8; 65536];
            let received = unsafe {
                windows_sys::Win32::Networking::WinSock::recv(
                    sock,
                    buf.as_mut_ptr(),
                    buf.len() as i32,
                    0,
                )
            };
            if received <= 0 {
                return Ok(None);
            }
            let packet = QuicCamouflagePacket::decode_masked(&buf[..received as usize])
                .map_err(|_| TransportError::Frame(crate::transport::FrameError::Malformed))?;
            return Ok(Some(Frame {
                frame_type: crate::transport::FrameType::Data,
                sequence: u64::from(packet.packet_number),
                payload: packet.payload,
            }));
        }

        let packet = match self.inbound.pop_front() {
            Some(masked) => QuicCamouflagePacket::decode_masked(&masked)
                .map_err(|_| TransportError::Frame(crate::transport::FrameError::Malformed))?,
            None => return Ok(None),
        };

        Ok(Some(Frame {
            frame_type: crate::transport::FrameType::Data,
            sequence: u64::from(packet.packet_number),
            payload: packet.payload,
        }))
    }
}

impl Drop for QuicMaskTransport {
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

#[cfg(test)]
mod tests {
    use crate::transport::mode::AttemptOutcome;
    use crate::transport::quic_mask::{QuicMaskConnectPolicy, QuicMaskTransport};
    use crate::transport::{Frame, FrameType, TransportStrategy};
    use core::time::Duration;

    #[test]
    fn quic_mask_connect_success_transitions_state() {
        let mut transport = QuicMaskTransport::new(QuicMaskConnectPolicy::Success);
        let outcome = transport.connect(Duration::from_secs(1)).expect("connect");

        assert_eq!(AttemptOutcome::Connected, outcome);
    }

    #[test]
    fn quic_mask_send_recv_roundtrip() {
        let mut transport = QuicMaskTransport::new(QuicMaskConnectPolicy::Success);
        transport.connect(Duration::from_secs(1)).expect("connect");

        let outbound_frame = Frame {
            frame_type: FrameType::Data,
            sequence: 0,
            payload: b"masked-payload".to_vec(),
        };
        transport.send(outbound_frame.clone()).expect("send");
        transport
            .queue_inbound(outbound_frame)
            .expect("queue inbound");
        let received = transport.recv().expect("recv").expect("frame");

        assert_eq!(b"masked-payload".to_vec(), received.payload);
    }
}
