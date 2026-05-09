use crate::crypto::kx::{derive_public_key, derive_shared_secret};
use crate::crypto::rng::os_seed;
use crate::crypto::sign::verify_message;
use crate::noise::SecurityError;
use crate::noise::state::{HandshakeState, NoiseSession};
use crate::noise::symmetric_state::SymmetricState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeMessage {
    ClientHello { ephemeral_public: [u8; 32] },
    ServerHello { ephemeral_public: [u8; 32] },
    AuthProof { signature: [u8; 64] },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeMachine {
    pub session: NoiseSession,
    pub symmetric_state: SymmetricState,
    local_ephemeral_secret: [u8; 32],
    peer_ephemeral_public: [u8; 32],
    peer_static_public: [u8; 32],
    seen_client_hello: bool,
    seen_server_hello: bool,
}

impl HandshakeMachine {
    pub fn new(peer_static_public: [u8; 32]) -> Self {
        Self {
            session: NoiseSession::default(),
            symmetric_state: SymmetricState::default(),
            local_ephemeral_secret: os_seed(),
            peer_ephemeral_public: [0u8; 32],
            peer_static_public,
            seen_client_hello: false,
            seen_server_hello: false,
        }
    }

    pub fn local_ephemeral_public(&self) -> [u8; 32] {
        derive_public_key(self.local_ephemeral_secret)
    }

    pub fn process(&mut self, message: HandshakeMessage) -> Result<HandshakeState, SecurityError> {
        match message {
            HandshakeMessage::ClientHello { ephemeral_public } => {
                if self.seen_client_hello {
                    return Err(SecurityError::ReplayDetected);
                }
                if self.session.state != HandshakeState::Init {
                    return Err(SecurityError::InvalidHandshake);
                }

                self.peer_ephemeral_public = ephemeral_public;
                self.symmetric_state.mix_hash(&ephemeral_public);
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

                let peer_for_dh = if self.seen_client_hello && !self.seen_server_hello {
                    ephemeral_public
                } else {
                    self.peer_ephemeral_public
                };

                let shared = derive_shared_secret(self.local_ephemeral_secret, peer_for_dh)
                    .map_err(|_| SecurityError::KeyDerivationFailed)?;
                self.symmetric_state.mix_key(&shared)?;

                self.peer_ephemeral_public = ephemeral_public;
                self.seen_server_hello = true;
            }
            HandshakeMessage::AuthProof { signature } => {
                if self.session.state != HandshakeState::EphemeralExchanged {
                    return Err(SecurityError::InvalidHandshake);
                }
                if !self.seen_server_hello {
                    return Err(SecurityError::InvalidHandshake);
                }

                verify_message(
                    self.peer_static_public,
                    &self.symmetric_state.handshake_hash,
                    signature,
                )
                .map_err(|_| SecurityError::ConstantTimeVerificationFailed)?;

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
    use crate::crypto::kx::{derive_public_key, derive_shared_secret};
    use crate::crypto::sign::{derive_verifying_key, sign_message};
    use crate::noise::HandshakeState;
    use crate::noise::handshake::{HandshakeMachine, HandshakeMessage};

    fn make_keypair(seed: u8) -> ([u8; 32], [u8; 32]) {
        let secret = [seed; 32];
        let public = derive_public_key(secret);
        (secret, public)
    }

    #[test]
    fn handshake_reaches_established_with_real_dh_and_signature() {
        let server_signing_key = [42u8; 32];
        let server_verifying_key = derive_verifying_key(server_signing_key);

        let mut client = HandshakeMachine::new(server_verifying_key);
        let mut server = HandshakeMachine::new([0u8; 32]);

        let client_eph_pub = client.local_ephemeral_public();
        let server_eph_pub = server.local_ephemeral_public();

        assert_eq!(
            HandshakeState::EphemeralExchanged,
            server
                .process(HandshakeMessage::ClientHello {
                    ephemeral_public: client_eph_pub,
                })
                .expect("server processes client hello")
        );

        assert_eq!(
            HandshakeState::EphemeralExchanged,
            client
                .process(HandshakeMessage::ClientHello {
                    ephemeral_public: client_eph_pub,
                })
                .expect("client records own hello")
        );

        assert_eq!(
            HandshakeState::EphemeralExchanged,
            client
                .process(HandshakeMessage::ServerHello {
                    ephemeral_public: server_eph_pub,
                })
                .expect("client processes server hello")
        );

        let signature = sign_message(server_signing_key, &client.symmetric_state.handshake_hash);

        assert_eq!(
            HandshakeState::Established,
            client
                .process(HandshakeMessage::AuthProof { signature })
                .expect("client verifies auth proof")
        );
    }

    #[test]
    fn handshake_rejects_invalid_signature() {
        let server_signing_key = [42u8; 32];
        let server_verifying_key = derive_verifying_key(server_signing_key);

        let mut client = HandshakeMachine::new(server_verifying_key);
        let client_eph_pub = client.local_ephemeral_public();

        client
            .process(HandshakeMessage::ClientHello {
                ephemeral_public: client_eph_pub,
            })
            .expect("client hello");

        client
            .process(HandshakeMessage::ServerHello {
                ephemeral_public: [7u8; 32],
            })
            .expect("server hello");

        let bad_signature = [0xFFu8; 64];
        let result = client.process(HandshakeMessage::AuthProof {
            signature: bad_signature,
        });

        assert!(result.is_err());
    }

    #[test]
    fn handshake_rejects_invalid_order() {
        let mut machine = HandshakeMachine::new([0u8; 32]);

        let result = machine.process(HandshakeMessage::AuthProof {
            signature: [1_u8; 64],
        });

        assert!(result.is_err());
    }

    #[test]
    fn handshake_detects_replay() {
        let mut machine = HandshakeMachine::new([0u8; 32]);
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
        let mut machine = HandshakeMachine::new([0u8; 32]);
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

    #[test]
    fn dh_shared_secret_matches_both_sides() {
        let (alice_secret, _) = make_keypair(3);
        let (bob_secret, _) = make_keypair(9);

        let alice_public = derive_public_key(alice_secret);
        let bob_public = derive_public_key(bob_secret);

        let shared_a = derive_shared_secret(alice_secret, bob_public).expect("alice");
        let shared_b = derive_shared_secret(bob_secret, alice_public).expect("bob");

        assert_eq!(shared_a, shared_b);
    }
}
