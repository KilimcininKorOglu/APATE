use crate::noise::SecurityError;

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

impl NoiseSession {
    pub fn transition(&mut self, next_state: HandshakeState) -> Result<(), SecurityError> {
        let valid_transition = matches!(
            (self.state, next_state),
            (HandshakeState::Init, HandshakeState::EphemeralExchanged)
                | (
                    HandshakeState::EphemeralExchanged,
                    HandshakeState::Authenticated
                )
                | (HandshakeState::Authenticated, HandshakeState::Established)
                | (_, HandshakeState::Failed)
        );

        if !valid_transition {
            return Err(SecurityError::InvalidHandshake);
        }

        self.state = next_state;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::noise::state::{HandshakeState, NoiseSession};

    #[test]
    fn transition_rejects_invalid_state_order() {
        let mut session = NoiseSession::default();

        let result = session.transition(HandshakeState::Established);

        assert!(result.is_err());
        assert_eq!(HandshakeState::Init, session.state);
    }

    #[test]
    fn transition_accepts_expected_sequence() {
        let mut session = NoiseSession::default();

        assert!(
            session
                .transition(HandshakeState::EphemeralExchanged)
                .is_ok()
        );
        assert!(session.transition(HandshakeState::Authenticated).is_ok());
        assert!(session.transition(HandshakeState::Established).is_ok());
    }
}
