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
        self.outbound.push(encoded);
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Frame>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
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
