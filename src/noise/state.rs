use crate::noise::SecurityError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    Init,
    EphemeralExchanged,
    Authenticated,
    Established,
    Rekeying,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoiseSession {
    pub state: HandshakeState,
    pub key_epoch: u64,
    pub transcript_hash: [u8; 32],
}

impl Default for NoiseSession {
    fn default() -> Self {
        Self {
            state: HandshakeState::Init,
            key_epoch: 0,
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
                | (HandshakeState::Established, HandshakeState::Rekeying)
                | (HandshakeState::Rekeying, HandshakeState::Established)
                | (_, HandshakeState::Failed)
        );

        if !valid_transition {
            return Err(SecurityError::InvalidHandshake);
        }

        self.state = next_state;
        Ok(())
    }

    pub fn begin_rekey(&mut self) -> Result<(), SecurityError> {
        self.transition(HandshakeState::Rekeying)
    }

    pub fn finalize_rekey(&mut self) -> Result<u64, SecurityError> {
        self.transition(HandshakeState::Established)?;
        self.key_epoch = self.key_epoch.saturating_add(1);
        Ok(self.key_epoch)
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

    #[test]
    fn rekey_updates_epoch_after_establishment() {
        let mut session = NoiseSession::default();
        session
            .transition(HandshakeState::EphemeralExchanged)
            .expect("ephemeral");
        session
            .transition(HandshakeState::Authenticated)
            .expect("authenticated");
        session
            .transition(HandshakeState::Established)
            .expect("established");

        session.begin_rekey().expect("begin rekey");
        let epoch = session.finalize_rekey().expect("finalize rekey");

        assert_eq!(HandshakeState::Established, session.state);
        assert_eq!(1, epoch);
    }
}
