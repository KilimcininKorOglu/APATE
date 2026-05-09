use crate::transport::mode::AttemptOutcome;
use crate::transport::{Frame, FrameType, TransportError, TransportStrategy};
use bytes::BytesMut;
use core::time::Duration;
use quinn_proto::crypto::rustls::QuicClientConfig;
use quinn_proto::{
    ClientConfig, Connection, ConnectionHandle, DatagramEvent, Endpoint, EndpointConfig, Event,
    StreamEvent, StreamId,
};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuicMaskConnectPolicy {
    Success,
    Failure,
}

pub struct QuicTransport {
    connect_policy: QuicMaskConnectPolicy,
    connected: bool,
    endpoint: Option<Endpoint>,
    connection: Option<(ConnectionHandle, Connection)>,
    stream_id: Option<StreamId>,
    #[cfg(unix)]
    fd: Option<i32>,
    #[cfg(target_os = "windows")]
    fd: Option<usize>,
    endpoint_addr: Option<SocketAddr>,
    server_name: String,
    outbound: Vec<Vec<u8>>,
    inbound: VecDeque<Vec<u8>>,
}

impl std::fmt::Debug for QuicTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuicTransport")
            .field("connected", &self.connected)
            .field("stream_id", &self.stream_id)
            .finish_non_exhaustive()
    }
}

#[cfg(unix)]
fn udp_send(fd: i32, data: &[u8]) {
    unsafe {
        libc::send(fd, data.as_ptr().cast(), data.len(), 0);
    }
}

#[cfg(unix)]
fn udp_recv(fd: i32) -> Option<Vec<u8>> {
    let mut buf = [0u8; 65536];
    let n = unsafe { libc::recv(fd, buf.as_mut_ptr().cast(), buf.len(), 0) };
    if n > 0 {
        Some(buf[..n as usize].to_vec())
    } else {
        None
    }
}

fn drain_transmits(fd_val: Option<i32>, conn: &mut Connection) {
    let mut buf = Vec::with_capacity(1500);
    while let Some(transmit) = conn.poll_transmit(Instant::now(), 1, &mut buf) {
        #[cfg(unix)]
        if let Some(fd) = fd_val {
            udp_send(fd, &buf[..transmit.size]);
        }
        let _ = transmit;
        buf.clear();
    }
}

impl QuicTransport {
    pub fn new(connect_policy: QuicMaskConnectPolicy) -> Self {
        Self {
            connect_policy,
            connected: false,
            endpoint: None,
            connection: None,
            stream_id: None,
            #[cfg(any(unix, target_os = "windows"))]
            fd: None,
            endpoint_addr: None,
            server_name: String::from("localhost"),
            outbound: Vec::new(),
            inbound: VecDeque::new(),
        }
    }

    pub fn set_endpoint(&mut self, endpoint: String) {
        self.endpoint_addr = endpoint.parse().ok();
    }

    pub fn set_server_name(&mut self, name: String) {
        self.server_name = name;
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

    fn fd_val(&self) -> Option<i32> {
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
        if let Some(remote) = self.endpoint_addr {
            return self.connect_real(remote);
        }

        match self.connect_policy {
            QuicMaskConnectPolicy::Success => {
                self.connected = true;
                Ok(AttemptOutcome::Connected)
            }
            QuicMaskConnectPolicy::Failure => Ok(AttemptOutcome::Failed),
        }
    }

    fn build_client_config() -> Result<ClientConfig, TransportError> {
        let crypto_provider = Arc::new(rustls::crypto::ring::default_provider());

        let rustls_config = rustls::ClientConfig::builder_with_provider(crypto_provider)
            .with_safe_default_protocol_versions()
            .map_err(|_| TransportError::NotConnected)?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();

        let quic_config =
            QuicClientConfig::try_from(rustls_config).map_err(|_| TransportError::NotConnected)?;

        Ok(ClientConfig::new(Arc::new(quic_config)))
    }

    fn connect_real(&mut self, remote: SocketAddr) -> Result<AttemptOutcome, TransportError> {
        let socket_fd = self.open_udp_socket(remote)?;
        if socket_fd < 0 {
            return Ok(AttemptOutcome::Failed);
        }

        let client_config = Self::build_client_config()?;
        let endpoint_config = EndpointConfig::default();
        let mut endpoint = Endpoint::new(Arc::new(endpoint_config), None, true, None);

        let now = Instant::now();
        let (handle, mut conn) = endpoint
            .connect(now, client_config, remote, &self.server_name)
            .map_err(|_| TransportError::NotConnected)?;

        drain_transmits(Some(socket_fd), &mut conn);

        self.endpoint = Some(endpoint);
        self.connection = Some((handle, conn));
        self.connected = true;
        Ok(AttemptOutcome::Connected)
    }

    #[cfg(unix)]
    fn open_udp_socket(&mut self, remote: SocketAddr) -> Result<i32, TransportError> {
        let fd = unsafe {
            libc::socket(
                if remote.is_ipv4() {
                    libc::AF_INET
                } else {
                    libc::AF_INET6
                },
                libc::SOCK_DGRAM,
                0,
            )
        };
        if fd < 0 {
            return Ok(-1);
        }
        unsafe {
            libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK);
        }
        self.fd = Some(fd);
        Ok(fd)
    }

    #[cfg(not(unix))]
    fn open_udp_socket(&mut self, _remote: SocketAddr) -> Result<i32, TransportError> {
        Ok(-1)
    }

    pub fn tick(&mut self) {
        let fd = self.fd_val();

        #[cfg(unix)]
        if let Some(fd_raw) = fd {
            while let Some(data) = udp_recv(fd_raw) {
                let now = Instant::now();
                let remote = self
                    .endpoint_addr
                    .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap());
                let mut response_buf = Vec::new();

                if let Some(ref mut endpoint) = self.endpoint
                    && let Some(event) = endpoint.handle(
                        now,
                        remote,
                        None,
                        None,
                        BytesMut::from(&data[..]),
                        &mut response_buf,
                    )
                    && let Some((handle, ref mut conn)) = self.connection
                {
                    match event {
                        DatagramEvent::ConnectionEvent(ch, ce) if ch == handle => {
                            conn.handle_event(ce);
                        }
                        DatagramEvent::Response(transmit) => {
                            udp_send(fd_raw, &response_buf[..transmit.size]);
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some((handle, ref mut conn)) = self.connection {
            let now = Instant::now();

            if let Some(deadline) = conn.poll_timeout()
                && now >= deadline
            {
                conn.handle_timeout(now);
            }

            drain_transmits(fd, conn);

            if let Some(ref mut endpoint) = self.endpoint {
                while let Some(ep_event) = conn.poll_endpoint_events() {
                    if let Some(conn_event) = endpoint.handle_event(handle, ep_event) {
                        conn.handle_event(conn_event);
                    }
                }
            }

            while let Some(event) = conn.poll() {
                match event {
                    Event::Connected => {
                        self.connected = true;
                    }
                    Event::Stream(StreamEvent::Readable { id }) => {
                        if self.stream_id.is_none() {
                            self.stream_id = Some(id);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn queue_inbound(&mut self, data: Vec<u8>) {
        self.inbound.push_back(data);
    }
}

impl TransportStrategy for QuicTransport {
    fn send(&mut self, frame: Frame) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        let fd = self.fd_val();
        if let Some((_, ref mut conn)) = self.connection {
            let stream_id = match self.stream_id {
                Some(id) => id,
                None => {
                    let id = conn
                        .streams()
                        .open(quinn_proto::Dir::Bi)
                        .ok_or(TransportError::NotConnected)?;
                    self.stream_id = Some(id);
                    id
                }
            };

            let mut send = conn.send_stream(stream_id);
            let written = send
                .write(&frame.payload)
                .map_err(|_| TransportError::NotConnected)?;
            if written == 0 {
                return Err(TransportError::NotConnected);
            }

            drain_transmits(fd, conn);
            return Ok(());
        }

        self.outbound.push(frame.payload);
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Frame>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        self.tick();

        if let Some((_, ref mut conn)) = self.connection
            && let Some(stream_id) = self.stream_id
        {
            let mut recv = conn.recv_stream(stream_id);
            let mut chunks = match recv.read(true) {
                Ok(c) => c,
                Err(_) => return Ok(None),
            };

            if let Ok(Some(chunk)) = chunks.next(1500) {
                let payload = chunk.bytes.to_vec();
                let _ = chunks.finalize();
                return Ok(Some(Frame {
                    frame_type: FrameType::Data,
                    sequence: 0,
                    payload,
                }));
            }
            let _ = chunks.finalize();
        }

        if let Some(data) = self.inbound.pop_front() {
            return Ok(Some(Frame {
                frame_type: FrameType::Data,
                sequence: 0,
                payload: data,
            }));
        }

        Ok(None)
    }
}

impl Drop for QuicTransport {
    fn drop(&mut self) {
        if let Some((_, ref mut conn)) = self.connection {
            conn.close(
                Instant::now(),
                quinn_proto::VarInt::from_u32(0),
                bytes::Bytes::from_static(b"done"),
            );
        }
        #[cfg(unix)]
        if let Some(fd) = self.fd {
            unsafe {
                libc::close(fd);
            }
        }
    }
}

#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;

    #[test]
    fn quic_transport_connect_policy_success() {
        let mut transport = QuicTransport::new(QuicMaskConnectPolicy::Success);
        let outcome = transport.connect(Duration::from_secs(1)).expect("connect");
        assert_eq!(AttemptOutcome::Connected, outcome);
    }

    #[test]
    fn quic_transport_connect_policy_failure() {
        let mut transport = QuicTransport::new(QuicMaskConnectPolicy::Failure);
        let outcome = transport.connect(Duration::from_secs(1)).expect("connect");
        assert_eq!(AttemptOutcome::Failed, outcome);
    }

    #[test]
    fn quic_transport_inbound_fallback() {
        let mut transport = QuicTransport::new(QuicMaskConnectPolicy::Success);
        transport.connect(Duration::from_secs(1)).expect("connect");
        transport.queue_inbound(b"test-data".to_vec());

        let frame = transport.recv().expect("recv").expect("frame");
        assert_eq!(b"test-data".to_vec(), frame.payload);
    }
}
