pub mod frame;

pub use frame::{
    DecodedFrame, FRAME_HEADER_LEN, Frame, FrameType, MAX_FRAME_PAYLOAD_LEN, PacketContext,
    decode_frame, encode_frame,
};

use thiserror::Error;

pub trait TransportStrategy {
    fn send(&mut self, frame: Frame) -> Result<(), TransportError>;
    fn recv(&mut self) -> Result<Option<Frame>, TransportError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FrameError {
    #[error("malformed frame")]
    Malformed,
    #[error("unsupported frame type")]
    UnsupportedType,
    #[error("frame payload exceeds limit")]
    PayloadTooLarge,
    #[error("invalid frame flags")]
    InvalidFlags,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransportError {
    #[error("transport not connected")]
    NotConnected,
    #[error("transport timed out")]
    Timeout,
    #[error("frame error: {0}")]
    Frame(#[from] FrameError),
}

#[cfg(test)]
mod tests {
    use crate::transport::{FrameError, TransportError};

    #[test]
    fn transport_error_wraps_frame_error() {
        let error = TransportError::from(FrameError::Malformed);

        assert_eq!("frame error: malformed frame", error.to_string());
    }
}
