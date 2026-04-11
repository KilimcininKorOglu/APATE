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
    outbound: Vec<Vec<u8>>,
    inbound: VecDeque<Vec<u8>>,
}

impl QuicMaskTransport {
    pub fn new(connect_policy: QuicMaskConnectPolicy) -> Self {
        Self {
            connect_policy,
            connected: false,
            outbound: Vec::new(),
            inbound: VecDeque::new(),
        }
    }

    pub fn connect(&mut self, _timeout: Duration) -> Result<AttemptOutcome, TransportError> {
        match self.connect_policy {
            QuicMaskConnectPolicy::Success => {
                self.connected = true;
                Ok(AttemptOutcome::Connected)
            }
            QuicMaskConnectPolicy::Failure => Ok(AttemptOutcome::Failed),
        }
    }

    pub fn mask_payload(payload: &[u8]) -> Vec<u8> {
        payload
            .iter()
            .enumerate()
            .map(|(idx, value)| value ^ ((idx as u8).wrapping_mul(31)))
            .collect()
    }

    pub fn unmask_payload(masked_payload: &[u8]) -> Vec<u8> {
        Self::mask_payload(masked_payload)
    }
}

impl TransportStrategy for QuicMaskTransport {
    fn send(&mut self, frame: Frame) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        self.outbound.push(Self::mask_payload(&frame.payload));
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Frame>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        let payload = match self.inbound.pop_front() {
            Some(masked) => Self::unmask_payload(&masked),
            None => return Ok(None),
        };

        Ok(Some(Frame {
            frame_type: crate::transport::FrameType::Data,
            sequence: 0,
            payload,
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::transport::quic_mask::QuicMaskTransport;

    #[test]
    fn quic_mask_payload_roundtrip() {
        let payload = b"masked-payload";
        let masked = QuicMaskTransport::mask_payload(payload);
        let unmasked = QuicMaskTransport::unmask_payload(&masked);

        assert_eq!(payload.to_vec(), unmasked);
    }
}
