use thiserror::Error;

const QUIC_MASK_FLAGS: u8 = 0b1100_0000;
const HEADER_LEN: usize = 7;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuicCamouflagePacket {
    pub connection_id: u32,
    pub packet_number: u16,
    pub payload: Vec<u8>,
}

impl QuicCamouflagePacket {
    pub fn encode_masked(&self) -> Result<Vec<u8>, QuicCamouflageError> {
        if self.payload.is_empty() || self.payload.len() > u16::MAX as usize {
            return Err(QuicCamouflageError::InvalidPayloadLength);
        }

        let payload_len = u16::try_from(self.payload.len())
            .map_err(|_| QuicCamouflageError::InvalidPayloadLength)?;
        let mut packet = Vec::with_capacity(HEADER_LEN + self.payload.len());
        packet.push(QUIC_MASK_FLAGS);
        packet.extend_from_slice(&self.connection_id.to_be_bytes());
        packet.extend_from_slice(&self.packet_number.to_be_bytes());

        let mask_key = derive_mask_key(self.connection_id, self.packet_number);
        let mut masked_payload = self.payload.clone();
        for (index, byte) in masked_payload.iter_mut().enumerate() {
            *byte ^= mask_key.wrapping_add(index as u8);
        }

        packet.extend_from_slice(&payload_len.to_be_bytes());
        packet.extend_from_slice(&masked_payload);
        Ok(packet)
    }

    pub fn decode_masked(packet: &[u8]) -> Result<Self, QuicCamouflageError> {
        if packet.len() < HEADER_LEN + 2 {
            return Err(QuicCamouflageError::MalformedPacket);
        }
        if packet[0] != QUIC_MASK_FLAGS {
            return Err(QuicCamouflageError::MalformedPacket);
        }

        let connection_id = u32::from_be_bytes([packet[1], packet[2], packet[3], packet[4]]);
        let packet_number = u16::from_be_bytes([packet[5], packet[6]]);
        let payload_len = u16::from_be_bytes([packet[7], packet[8]]) as usize;
        if packet.len() != HEADER_LEN + 2 + payload_len {
            return Err(QuicCamouflageError::MalformedPacket);
        }

        let mask_key = derive_mask_key(connection_id, packet_number);
        let mut payload = packet[9..].to_vec();
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask_key.wrapping_add(index as u8);
        }

        Ok(Self {
            connection_id,
            packet_number,
            payload,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum QuicCamouflageError {
    #[error("invalid quic camouflage payload length")]
    InvalidPayloadLength,
    #[error("malformed quic camouflage packet")]
    MalformedPacket,
}

fn derive_mask_key(connection_id: u32, packet_number: u16) -> u8 {
    let cid_fold = (connection_id as u8)
        ^ ((connection_id >> 8) as u8)
        ^ ((connection_id >> 16) as u8)
        ^ ((connection_id >> 24) as u8);
    cid_fold ^ (packet_number as u8) ^ ((packet_number >> 8) as u8)
}

#[cfg(test)]
mod tests {
    use crate::stealth::quic_camouflage::QuicCamouflagePacket;

    #[test]
    fn quic_camouflage_packet_roundtrip() {
        let packet = QuicCamouflagePacket {
            connection_id: 42,
            packet_number: 7,
            payload: b"hello-quic".to_vec(),
        };

        let encoded = packet.encode_masked().expect("encoded");
        let decoded = QuicCamouflagePacket::decode_masked(&encoded).expect("decoded");

        assert_eq!(packet, decoded);
    }
}
