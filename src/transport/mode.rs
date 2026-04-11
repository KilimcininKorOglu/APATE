use crate::util::TransportMode;
use core::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    UdpTls,
    TcpTls,
    QuicMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptOutcome {
    Connected,
    TimedOut,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeNegotiator {
    mode: TransportMode,
    fallback_timeout: Duration,
}

impl ModeNegotiator {
    pub fn new(mode: TransportMode, fallback_timeout_secs: u16) -> Self {
        Self {
            mode,
            fallback_timeout: Duration::from_secs(u64::from(fallback_timeout_secs)),
        }
    }

    pub fn fallback_timeout(&self) -> Duration {
        self.fallback_timeout
    }

    pub fn initial_kind(&self) -> TransportKind {
        match self.mode {
            TransportMode::Auto | TransportMode::Udp => TransportKind::UdpTls,
            TransportMode::Tcp => TransportKind::TcpTls,
            TransportMode::QuicMask => TransportKind::QuicMask,
        }
    }

    pub fn next_kind(
        &self,
        current: TransportKind,
        outcome: AttemptOutcome,
    ) -> Option<TransportKind> {
        match (self.mode, current, outcome) {
            (TransportMode::Auto, TransportKind::UdpTls, AttemptOutcome::TimedOut) => {
                Some(TransportKind::TcpTls)
            }
            (TransportMode::Auto, TransportKind::UdpTls, AttemptOutcome::Failed) => {
                Some(TransportKind::TcpTls)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::transport::mode::{AttemptOutcome, ModeNegotiator, TransportKind};
    use crate::util::TransportMode;

    #[test]
    fn auto_mode_falls_back_to_tcp_on_udp_timeout() {
        let negotiator = ModeNegotiator::new(TransportMode::Auto, 3);

        assert_eq!(
            Some(TransportKind::TcpTls),
            negotiator.next_kind(TransportKind::UdpTls, AttemptOutcome::TimedOut)
        );
    }

    #[test]
    fn forced_tcp_mode_bypasses_fallback() {
        let negotiator = ModeNegotiator::new(TransportMode::Tcp, 3);

        assert_eq!(
            None,
            negotiator.next_kind(TransportKind::TcpTls, AttemptOutcome::TimedOut)
        );
    }

    #[test]
    fn quic_mask_mode_uses_quic_mask_transport_kind() {
        let negotiator = ModeNegotiator::new(TransportMode::QuicMask, 3);
        assert_eq!(TransportKind::QuicMask, negotiator.initial_kind());
    }
}
