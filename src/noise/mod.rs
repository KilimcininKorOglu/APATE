use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    Init,
    EphemeralExchanged,
    Authenticated,
    Established,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoiseSession {
    pub state: HandshakeState,
    pub transcript_hash: [u8; 32],
}

impl Default for NoiseSession {
    fn default() -> Self {
        Self {
            state: HandshakeState::Init,
            transcript_hash: [0_u8; 32],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SecurityError {
    #[error("invalid handshake sequence")]
    InvalidHandshake,
    #[error("replay detected")]
    ReplayDetected,
    #[error("key derivation failed")]
    KeyDerivationFailed,
    #[error("constant-time verification failed")]
    ConstantTimeVerificationFailed,
}

#[cfg(test)]
mod tests {
    use crate::noise::{HandshakeState, NoiseSession};

    #[test]
    fn default_noise_session_is_init() {
        let session = NoiseSession::default();

        assert_eq!(HandshakeState::Init, session.state);
    }
}
