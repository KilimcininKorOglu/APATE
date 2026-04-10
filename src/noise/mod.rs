pub mod cipher_state;
pub mod handshake;
pub mod state;
pub mod symmetric_state;

pub use state::{HandshakeState, NoiseSession};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SecurityError {
    #[error("invalid handshake sequence")]
    InvalidHandshake,
    #[error("replay detected")]
    ReplayDetected,
    #[error("key derivation failed")]
    KeyDerivationFailed,
    #[error("cipher operation failed")]
    CipherFailure,
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
