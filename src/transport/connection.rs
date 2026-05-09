#[path = "migration.rs"]
mod migration;

use crate::crypto::rng::os_seed;
use crate::noise::handshake::{HandshakeMachine, HandshakeMessage};
use crate::transport::fec::{FecController, FecMode};
use crate::transport::mode::{AttemptOutcome, ModeNegotiator, TransportKind};
use crate::transport::pacing::CompressionPolicy;
use crate::transport::quic_mask::{QuicMaskConnectPolicy, QuicMaskTransport};
use crate::transport::tcp_tls::TcpTlsTransport;
use crate::transport::udp_tls::UdpTlsTransport;
use crate::transport::{Frame, FrameError, FrameType, TransportError, TransportStrategy};
use crate::util::ConnectionState;
use core::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct TransportEngine {
    state: ConnectionState,
    sequence: u64,
    key_epoch: u64,
    fallback_count: u32,
    negotiator: ModeNegotiator,
    active_kind: TransportKind,
    session_id: [u8; 16],
    migration_secret: [u8; 32],
    endpoint: String,
    fec_controller: FecController,
    compression_policy: CompressionPolicy,
    udp: UdpTlsTransport,
    tcp: TcpTlsTransport,
    quic: QuicMaskTransport,
    handshake: Option<HandshakeMachine>,
}

impl TransportEngine {
    fn connect_kind(
        &mut self,
        kind: TransportKind,
        timeout: Duration,
    ) -> Result<AttemptOutcome, TransportError> {
        match kind {
            TransportKind::UdpTls => self.udp.connect(timeout),
            TransportKind::TcpTls => self.tcp.connect(timeout),
            TransportKind::QuicMask => self.quic.connect(timeout),
        }
    }

    pub fn new(negotiator: ModeNegotiator, udp: UdpTlsTransport, tcp: TcpTlsTransport) -> Self {
        let active_kind = negotiator.initial_kind();
        let mut session_id = [0u8; 16];
        let seed = os_seed();
        session_id.copy_from_slice(&seed[..16]);
        let mut migration_secret = [0u8; 32];
        migration_secret.copy_from_slice(&os_seed());
        Self {
            state: ConnectionState::Init,
            sequence: 0,
            key_epoch: 0,
            fallback_count: 0,
            negotiator,
            active_kind,
            session_id,
            migration_secret,
            endpoint: String::from("127.0.0.1:443"),
            fec_controller: FecController::default(),
            compression_policy: CompressionPolicy::default(),
            udp,
            tcp,
            quic: QuicMaskTransport::new(QuicMaskConnectPolicy::Success),
            handshake: None,
        }
    }

    pub fn set_handshake(&mut self, handshake: HandshakeMachine) {
        self.handshake = Some(handshake);
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn active_kind(&self) -> TransportKind {
        self.active_kind
    }

    pub fn fallback_count(&self) -> u32 {
        self.fallback_count
    }

    pub fn key_epoch(&self) -> u64 {
        self.key_epoch
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn active_raw_fd(&self) -> Option<i32> {
        match self.active_kind {
            TransportKind::UdpTls => self.udp.raw_fd(),
            TransportKind::TcpTls => self.tcp.raw_fd(),
            TransportKind::QuicMask => self.quic.raw_fd(),
        }
    }

    pub fn fec_mode(&self) -> FecMode {
        self.fec_controller.mode()
    }

    pub fn update_observed_loss(&mut self, loss_rate: f32) {
        self.fec_controller
            .update_loss_rate(loss_rate, self.active_kind);
    }

    pub fn build_fec_parity(&self, data_shards: &[Vec<u8>]) -> Vec<Vec<u8>> {
        self.fec_controller.build_parity_shards(data_shards)
    }

    pub fn migration_proof(&self, new_endpoint: &str) -> [u8; 16] {
        migration::generate_migration_proof(self.session_id, &self.migration_secret, new_endpoint)
    }

    pub fn rekey(&mut self) -> Result<u64, TransportError> {
        if self.state != ConnectionState::Established {
            return Err(TransportError::NotConnected);
        }

        self.state = ConnectionState::Rekeying;
        self.key_epoch = self.key_epoch.saturating_add(1);
        self.state = ConnectionState::Established;
        Ok(self.key_epoch)
    }

    pub fn migrate_endpoint(
        &mut self,
        new_endpoint: String,
        proof: [u8; 16],
    ) -> Result<(), TransportError> {
        if self.state != ConnectionState::Established {
            return Err(TransportError::NotConnected);
        }

        self.state = ConnectionState::Migrating;
        let is_valid = migration::validate_migration_proof(
            self.session_id,
            &self.migration_secret,
            &new_endpoint,
            proof,
        );
        if !is_valid {
            self.state = ConnectionState::Established;
            return Err(TransportError::Frame(FrameError::Malformed));
        }

        self.endpoint = new_endpoint;
        self.state = ConnectionState::Established;
        Ok(())
    }

    pub fn establish(&mut self) -> Result<(), TransportError> {
        self.state = ConnectionState::Handshaking;
        let timeout = self.negotiator.fallback_timeout();
        let mut attempt_kind = self.active_kind;

        loop {
            match self.connect_kind(attempt_kind, timeout)? {
                AttemptOutcome::Connected => {
                    self.active_kind = attempt_kind;
                    self.run_handshake()?;
                    self.state = ConnectionState::Established;
                    return Ok(());
                }
                outcome => {
                    let Some(next_kind) = self.negotiator.next_kind(attempt_kind, outcome) else {
                        self.state = ConnectionState::Closed;
                        return Err(TransportError::Timeout);
                    };
                    if next_kind == attempt_kind {
                        self.state = ConnectionState::Closed;
                        return Err(TransportError::Timeout);
                    }

                    attempt_kind = next_kind;
                    self.active_kind = next_kind;
                    self.fallback_count = self.fallback_count.saturating_add(1);
                }
            }
        }
    }

    fn run_handshake(&mut self) -> Result<(), TransportError> {
        let Some(ref mut machine) = self.handshake else {
            return Ok(());
        };

        let local_pub = machine.local_ephemeral_public();
        machine
            .process(HandshakeMessage::ClientHello {
                ephemeral_public: local_pub,
            })
            .map_err(|_| TransportError::NotConnected)?;

        Ok(())
    }

    pub fn send_payload(&mut self, payload: Vec<u8>) -> Result<u64, TransportError> {
        if self.state != ConnectionState::Established {
            return Err(TransportError::NotConnected);
        }

        let sequence = self.sequence;
        self.sequence = self.sequence.saturating_add(1);
        let payload = self
            .compression_policy
            .compress_rle(&payload)
            .unwrap_or(payload);
        let frame = Frame {
            frame_type: FrameType::Data,
            sequence,
            payload,
        };
        self.send_frame(frame)?;
        Ok(sequence)
    }

    pub fn send_frame(&mut self, frame: Frame) -> Result<(), TransportError> {
        match self.active_kind {
            TransportKind::UdpTls => self.udp.send(frame),
            TransportKind::TcpTls => self.tcp.send(frame),
            TransportKind::QuicMask => self.quic.send(frame),
        }
    }

    pub fn recv_frame(&mut self) -> Result<Option<Frame>, TransportError> {
        match self.active_kind {
            TransportKind::UdpTls => self.udp.recv(),
            TransportKind::TcpTls => self.tcp.recv(),
            TransportKind::QuicMask => self.quic.recv(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::transport::FecMode;
    use crate::transport::connection::TransportEngine;
    use crate::transport::mode::{ModeNegotiator, TransportKind};
    use crate::transport::tcp_tls::{TcpConnectPolicy, TcpTlsTransport};
    use crate::transport::udp_tls::{UdpConnectPolicy, UdpTlsTransport};
    use crate::util::{ConnectionState, TransportMode};

    #[test]
    fn auto_mode_falls_back_to_tcp_when_udp_times_out() {
        let negotiator = ModeNegotiator::new(TransportMode::Auto, 3);
        let udp = UdpTlsTransport::new(UdpConnectPolicy::Timeout);
        let tcp = TcpTlsTransport::new(TcpConnectPolicy::Success);
        let mut engine = TransportEngine::new(negotiator, udp, tcp);

        engine.establish().expect("auto fallback establish");

        assert_eq!(ConnectionState::Established, engine.state());
        assert_eq!(TransportKind::TcpTls, engine.active_kind());
        assert_eq!(1, engine.fallback_count());
    }

    #[test]
    fn auto_mode_falls_back_to_tcp_when_udp_fails() {
        let negotiator = ModeNegotiator::new(TransportMode::Auto, 3);
        let udp = UdpTlsTransport::new(UdpConnectPolicy::Failure);
        let tcp = TcpTlsTransport::new(TcpConnectPolicy::Success);
        let mut engine = TransportEngine::new(negotiator, udp, tcp);

        engine.establish().expect("auto fallback establish");

        assert_eq!(ConnectionState::Established, engine.state());
        assert_eq!(TransportKind::TcpTls, engine.active_kind());
        assert_eq!(1, engine.fallback_count());
    }

    #[test]
    fn auto_mode_fails_when_fallback_transport_fails() {
        let negotiator = ModeNegotiator::new(TransportMode::Auto, 3);
        let udp = UdpTlsTransport::new(UdpConnectPolicy::Timeout);
        let tcp = TcpTlsTransport::new(TcpConnectPolicy::Failure);
        let mut engine = TransportEngine::new(negotiator, udp, tcp);

        let result = engine.establish();

        assert!(result.is_err());
        assert_eq!(ConnectionState::Closed, engine.state());
        assert_eq!(TransportKind::TcpTls, engine.active_kind());
        assert_eq!(1, engine.fallback_count());
    }

    #[test]
    fn forced_udp_mode_bypasses_fallback_path() {
        let negotiator = ModeNegotiator::new(TransportMode::Udp, 3);
        let udp = UdpTlsTransport::new(UdpConnectPolicy::Success);
        let tcp = TcpTlsTransport::new(TcpConnectPolicy::Failure);
        let mut engine = TransportEngine::new(negotiator, udp, tcp);

        engine.establish().expect("forced udp establish");

        assert_eq!(ConnectionState::Established, engine.state());
        assert_eq!(TransportKind::UdpTls, engine.active_kind());
        assert_eq!(0, engine.fallback_count());
    }

    #[test]
    fn quic_mask_mode_establishes_without_fallback() {
        let negotiator = ModeNegotiator::new(TransportMode::QuicMask, 3);
        let udp = UdpTlsTransport::new(UdpConnectPolicy::Timeout);
        let tcp = TcpTlsTransport::new(TcpConnectPolicy::Failure);
        let mut engine = TransportEngine::new(negotiator, udp, tcp);

        engine.establish().expect("forced quic establish");

        assert_eq!(ConnectionState::Established, engine.state());
        assert_eq!(TransportKind::QuicMask, engine.active_kind());
        assert_eq!(0, engine.fallback_count());
    }

    #[test]
    fn rekey_increments_epoch_and_keeps_established_state() {
        let negotiator = ModeNegotiator::new(TransportMode::Udp, 3);
        let udp = UdpTlsTransport::new(UdpConnectPolicy::Success);
        let tcp = TcpTlsTransport::new(TcpConnectPolicy::Failure);
        let mut engine = TransportEngine::new(negotiator, udp, tcp);
        engine.establish().expect("establish");

        let epoch = engine.rekey().expect("rekey");

        assert_eq!(1, epoch);
        assert_eq!(ConnectionState::Established, engine.state());
    }

    #[test]
    fn migration_updates_endpoint_with_valid_proof() {
        let negotiator = ModeNegotiator::new(TransportMode::Udp, 3);
        let udp = UdpTlsTransport::new(UdpConnectPolicy::Success);
        let tcp = TcpTlsTransport::new(TcpConnectPolicy::Failure);
        let mut engine = TransportEngine::new(negotiator, udp, tcp);
        engine.establish().expect("establish");
        let next_endpoint = String::from("198.51.100.20:443");
        let proof = engine.migration_proof(&next_endpoint);

        engine
            .migrate_endpoint(next_endpoint.clone(), proof)
            .expect("migrate");

        assert_eq!(next_endpoint, engine.endpoint());
        assert_eq!(ConnectionState::Established, engine.state());
    }

    #[test]
    fn tcp_transport_keeps_fec_disabled_even_under_high_loss() {
        let negotiator = ModeNegotiator::new(TransportMode::Tcp, 3);
        let udp = UdpTlsTransport::new(UdpConnectPolicy::Timeout);
        let tcp = TcpTlsTransport::new(TcpConnectPolicy::Success);
        let mut engine = TransportEngine::new(negotiator, udp, tcp);
        engine.establish().expect("tcp establish");

        engine.update_observed_loss(0.35);

        assert_eq!(FecMode::Disabled, engine.fec_mode());
    }
}
