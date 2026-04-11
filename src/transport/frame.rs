use crate::transport::FrameError;
use crate::util::buf::{ByteCursor, ByteWriter};

pub const FRAME_HEADER_LEN: usize = 12;
pub const MAX_FRAME_PAYLOAD_LEN: usize = 16 * 1024;
const SUPPORTED_FLAGS_MASK: u8 = 0b0000_0011;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    Handshake = 0,
    Data = 1,
    Ack = 2,
    Rekey = 3,
    Migrate = 4,
    Close = 5,
}

impl FrameType {
    pub fn from_u8(value: u8) -> Result<Self, FrameError> {
        match value {
            0 => Ok(Self::Handshake),
            1 => Ok(Self::Data),
            2 => Ok(Self::Ack),
            3 => Ok(Self::Rekey),
            4 => Ok(Self::Migrate),
            5 => Ok(Self::Close),
            _ => Err(FrameError::UnsupportedType),
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub frame_type: FrameType,
    pub sequence: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketContext {
    pub flags: u8,
    pub payload_len: u16,
    pub total_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedFrame {
    pub frame: Frame,
    pub context: PacketContext,
}

pub fn encode_frame(frame: &Frame, flags: u8) -> Result<Vec<u8>, FrameError> {
    if (flags & !SUPPORTED_FLAGS_MASK) != 0 {
        return Err(FrameError::InvalidFlags);
    }
    if frame.payload.len() > MAX_FRAME_PAYLOAD_LEN {
        return Err(FrameError::PayloadTooLarge);
    }

    let payload_len =
        u16::try_from(frame.payload.len()).map_err(|_error| FrameError::PayloadTooLarge)?;
    let mut writer = ByteWriter::with_capacity(FRAME_HEADER_LEN + usize::from(payload_len));
    writer.write_u8(frame.frame_type.to_u8());
    writer.write_u8(flags);
    writer.write_u16_be(payload_len);
    writer.write_u64_be(frame.sequence);
    writer.write_bytes(&frame.payload);
    Ok(writer.into_vec())
}

pub fn decode_frame(packet: &[u8]) -> Result<DecodedFrame, FrameError> {
    if packet.len() < FRAME_HEADER_LEN {
        return Err(FrameError::Malformed);
    }

    let mut cursor = ByteCursor::new(packet);
    let frame_type_raw = cursor.read_u8().ok_or(FrameError::Malformed)?;
    let flags = cursor.read_u8().ok_or(FrameError::Malformed)?;
    if (flags & !SUPPORTED_FLAGS_MASK) != 0 {
        return Err(FrameError::InvalidFlags);
    }

    let payload_len = cursor.read_u16_be().ok_or(FrameError::Malformed)?;
    if usize::from(payload_len) > MAX_FRAME_PAYLOAD_LEN {
        return Err(FrameError::PayloadTooLarge);
    }

    let sequence = cursor.read_u64_be().ok_or(FrameError::Malformed)?;
    let frame_type = FrameType::from_u8(frame_type_raw)?;
    let payload = cursor
        .read_exact(usize::from(payload_len))
        .ok_or(FrameError::Malformed)?
        .to_vec();
    if cursor.remaining() != 0 {
        return Err(FrameError::Malformed);
    }

    Ok(DecodedFrame {
        frame: Frame {
            frame_type,
            sequence,
            payload,
        },
        context: PacketContext {
            flags,
            payload_len,
            total_len: packet.len(),
        },
    })
}

#[cfg(test)]
mod tests {
    use crate::transport::FrameError;
    use crate::transport::frame::{Frame, FrameType, decode_frame, encode_frame};

    #[test]
    fn frame_roundtrip() {
        let frame = Frame {
            frame_type: FrameType::Data,
            sequence: 44,
            payload: b"hello".to_vec(),
        };

        let encoded = encode_frame(&frame, 0b01).expect("encode");
        let decoded = decode_frame(&encoded).expect("decode");

        assert_eq!(frame, decoded.frame);
        assert_eq!(0b01, decoded.context.flags);
    }

    #[test]
    fn frame_rejects_invalid_flags() {
        let frame = Frame {
            frame_type: FrameType::Data,
            sequence: 1,
            payload: vec![1],
        };

        let error = encode_frame(&frame, 0b1000_0000).expect_err("invalid flags");
        assert_eq!(FrameError::InvalidFlags, error);
    }
}
