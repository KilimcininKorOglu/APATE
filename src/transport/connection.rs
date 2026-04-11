use crate::transport::mode::{AttemptOutcome, ModeNegotiator, TransportKind};
use crate::transport::tcp_tls::TcpTlsTransport;
use crate::transport::udp_tls::UdpTlsTransport;
use crate::transport::{Frame, FrameType, TransportError, TransportStrategy};
use crate::util::ConnectionState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportEngine {
    state: ConnectionState,
    sequence: u64,
    fallback_count: u32,
    negotiator: ModeNegotiator,
    active_kind: TransportKind,
    udp: UdpTlsTransport,
    tcp: TcpTlsTransport,
}

impl TransportEngine {
    pub fn new(negotiator: ModeNegotiator, udp: UdpTlsTransport, tcp: TcpTlsTransport) -> Self {
        let active_kind = negotiator.initial_kind();
        Self {
            state: ConnectionState::Init,
            sequence: 0,
            fallback_count: 0,
            negotiator,
            active_kind,
            udp,
            tcp,
        }
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

    pub fn establish(&mut self) -> Result<(), TransportError> {
        self.state = ConnectionState::Handshaking;
        let timeout = self.negotiator.fallback_timeout();

        match self.active_kind {
            TransportKind::UdpTls => match self.udp.connect(timeout)? {
                AttemptOutcome::Connected => {}
                outcome => {
                    if let Some(next_kind) = self.negotiator.next_kind(self.active_kind, outcome) {
                        self.active_kind = next_kind;
                        self.fallback_count = self.fallback_count.saturating_add(1);
                        let tcp_outcome = self.tcp.connect(timeout)?;
                        if tcp_outcome != AttemptOutcome::Connected {
                            self.state = ConnectionState::Closed;
                            return Err(TransportError::Timeout);
                        }
                    } else {
                        self.state = ConnectionState::Closed;
                        return Err(TransportError::Timeout);
                    }
                }
            },
            TransportKind::TcpTls => {
                if self.tcp.connect(timeout)? != AttemptOutcome::Connected {
                    self.state = ConnectionState::Closed;
                    return Err(TransportError::Timeout);
                }
            }
            TransportKind::QuicMask => {
                self.state = ConnectionState::Closed;
                return Err(TransportError::NotConnected);
            }
        }

        self.state = ConnectionState::Established;
        Ok(())
    }

    pub fn send_payload(&mut self, payload: Vec<u8>) -> Result<u64, TransportError> {
        if self.state != ConnectionState::Established {
            return Err(TransportError::NotConnected);
        }

        let sequence = self.sequence;
        self.sequence = self.sequence.saturating_add(1);
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
            TransportKind::QuicMask => Err(TransportError::NotConnected),
        }
    }

    pub fn recv_frame(&mut self) -> Result<Option<Frame>, TransportError> {
        match self.active_kind {
            TransportKind::UdpTls => self.udp.recv(),
            TransportKind::TcpTls => self.tcp.recv(),
            TransportKind::QuicMask => Err(TransportError::NotConnected),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
