use crate::noise::SecurityError;
use crate::noise::state::{HandshakeState, NoiseSession};
use crate::noise::symmetric_state::SymmetricState;
use subtle::ConstantTimeEq;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeMessage {
    ClientHello { ephemeral_public: [u8; 32] },
    ServerHello { ephemeral_public: [u8; 32] },
    AuthProof { signature: [u8; 64] },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HandshakeMachine {
    pub session: NoiseSession,
    pub symmetric_state: SymmetricState,
    seen_client_hello: bool,
    seen_server_hello: bool,
}

impl HandshakeMachine {
    pub fn process(&mut self, message: HandshakeMessage) -> Result<HandshakeState, SecurityError> {
        match message {
            HandshakeMessage::ClientHello { ephemeral_public } => {
                if self.seen_client_hello {
                    return Err(SecurityError::ReplayDetected);
                }
                if self.session.state != HandshakeState::Init {
                    return Err(SecurityError::InvalidHandshake);
                }

                self.symmetric_state.mix_hash(&ephemeral_public);
                self.symmetric_state.mix_key(&ephemeral_public)?;
                self.seen_client_hello = true;
                self.session
                    .transition(HandshakeState::EphemeralExchanged)?;
            }
            HandshakeMessage::ServerHello { ephemeral_public } => {
                if self.seen_server_hello {
                    return Err(SecurityError::ReplayDetected);
                }
                if self.session.state != HandshakeState::EphemeralExchanged {
                    return Err(SecurityError::InvalidHandshake);
                }

                self.symmetric_state.mix_hash(&ephemeral_public);
                self.symmetric_state.mix_key(&ephemeral_public)?;
                self.seen_server_hello = true;
            }
            HandshakeMessage::AuthProof { signature } => {
                if self.session.state != HandshakeState::EphemeralExchanged {
                    return Err(SecurityError::InvalidHandshake);
                }
                if !self.seen_server_hello {
                    return Err(SecurityError::InvalidHandshake);
                }
                if signature.ct_eq(&[0_u8; 64]).into() {
                    return Err(SecurityError::ConstantTimeVerificationFailed);
                }

                self.symmetric_state.mix_hash(&signature);
                self.session.transition(HandshakeState::Authenticated)?;
                self.session.transition(HandshakeState::Established)?;
            }
        }

        Ok(self.session.state)
    }
}

#[cfg(test)]
mod tests {
    use crate::noise::HandshakeState;
    use crate::noise::handshake::{HandshakeMachine, HandshakeMessage};

    #[test]
    fn handshake_reaches_established_on_valid_sequence() {
        let mut machine = HandshakeMachine::default();

        assert_eq!(
            HandshakeState::EphemeralExchanged,
            machine
                .process(HandshakeMessage::ClientHello {
                    ephemeral_public: [1_u8; 32],
                })
                .expect("client hello")
        );
        assert_eq!(
            HandshakeState::EphemeralExchanged,
            machine
                .process(HandshakeMessage::ServerHello {
                    ephemeral_public: [2_u8; 32],
                })
                .expect("server hello")
        );
        assert_eq!(
            HandshakeState::Established,
            machine
                .process(HandshakeMessage::AuthProof {
                    signature: [9_u8; 64],
                })
                .expect("auth proof")
        );
    }

    #[test]
    fn handshake_rejects_invalid_order() {
        let mut machine = HandshakeMachine::default();

        let result = machine.process(HandshakeMessage::AuthProof {
            signature: [1_u8; 64],
        });

        assert!(result.is_err());
    }

    #[test]
    fn handshake_detects_replay() {
        let mut machine = HandshakeMachine::default();
        machine
            .process(HandshakeMessage::ClientHello {
                ephemeral_public: [1_u8; 32],
            })
            .expect("first client hello");

        let replay_result = machine.process(HandshakeMessage::ClientHello {
            ephemeral_public: [1_u8; 32],
        });

        assert!(replay_result.is_err());
    }

    #[test]
    fn handshake_rejects_auth_proof_before_server_hello() {
        let mut machine = HandshakeMachine::default();
        machine
            .process(HandshakeMessage::ClientHello {
                ephemeral_public: [1_u8; 32],
            })
            .expect("client hello");

        let result = machine.process(HandshakeMessage::AuthProof {
            signature: [9_u8; 64],
        });

        assert!(result.is_err());
    }
}
