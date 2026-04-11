use crate::auth::{AuthBackend, AuthError, AuthIdentity, AuthInput};
use crate::util::AuthMethod;

pub struct AuthCoordinator {
    enabled_methods: Vec<AuthMethod>,
    backends: Vec<(AuthMethod, Box<dyn AuthBackend + Send + Sync>)>,
}

impl AuthCoordinator {
    pub fn new(enabled_methods: Vec<AuthMethod>) -> Self {
        Self {
            enabled_methods,
            backends: Vec::new(),
        }
    }

    pub fn register_backend(
        &mut self,
        method: AuthMethod,
        backend: Box<dyn AuthBackend + Send + Sync>,
    ) {
        if let Some(index) = self
            .backends
            .iter()
            .position(|(candidate, _)| *candidate == method)
        {
            self.backends[index] = (method, backend);
            return;
        }

        self.backends.push((method, backend));
    }

    pub fn authenticate(&self, input: AuthInput) -> Result<AuthIdentity, AuthError> {
        if !self.enabled_methods.contains(&input.method) {
            return Err(AuthError::UnsupportedBackend {
                method: input.method,
            });
        }

        let backend = self
            .backends
            .iter()
            .find(|(method, _)| *method == input.method)
            .map(|(_, backend)| backend)
            .ok_or(AuthError::UnsupportedBackend {
                method: input.method,
            })?;

        backend.authenticate(input)
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::backend::AuthCoordinator;
    use crate::auth::static_key::StaticKeyBackend;
    use crate::auth::token::{TokenBackend, TokenClaims, TokenPolicy};
    use crate::auth::{AuthError, AuthInput};
    use crate::util::AuthMethod;

    fn token_signing_key() -> [u8; 32] {
        [9_u8; 32]
    }

    #[test]
    fn coordinator_dispatches_to_registered_backend() {
        let mut coordinator = AuthCoordinator::new(vec![AuthMethod::StaticKey]);
        coordinator.register_backend(
            AuthMethod::StaticKey,
            Box::new(StaticKeyBackend::new(vec![b"ok-key".to_vec()])),
        );

        let identity = coordinator
            .authenticate(AuthInput {
                method: AuthMethod::StaticKey,
                payload: b"ok-key".to_vec(),
            })
            .expect("coordinator auth");

        assert_eq!("static-key", identity.subject);
    }

    #[test]
    fn coordinator_rejects_when_backend_not_enabled() {
        let mut coordinator = AuthCoordinator::new(vec![AuthMethod::Token]);
        coordinator.register_backend(
            AuthMethod::StaticKey,
            Box::new(StaticKeyBackend::new(vec![b"ok-key".to_vec()])),
        );

        let error = coordinator
            .authenticate(AuthInput {
                method: AuthMethod::StaticKey,
                payload: b"ok-key".to_vec(),
            })
            .expect_err("unsupported method");

        assert_eq!(
            AuthError::UnsupportedBackend {
                method: AuthMethod::StaticKey,
            },
            error
        );
    }

    #[test]
    fn coordinator_preserves_sanitized_rejected_error() {
        let mut coordinator = AuthCoordinator::new(vec![AuthMethod::StaticKey]);
        coordinator.register_backend(
            AuthMethod::StaticKey,
            Box::new(StaticKeyBackend::new(vec![b"ok-key".to_vec()])),
        );

        let error = coordinator
            .authenticate(AuthInput {
                method: AuthMethod::StaticKey,
                payload: b"bad-key".to_vec(),
            })
            .expect_err("rejected");

        assert_eq!(AuthError::Rejected, error);
        assert_eq!("authentication rejected", error.to_string());
    }

    #[test]
    fn coordinator_supports_mixed_static_and_token_backends() {
        let mut coordinator = AuthCoordinator::new(vec![AuthMethod::StaticKey, AuthMethod::Token]);
        coordinator.register_backend(
            AuthMethod::StaticKey,
            Box::new(StaticKeyBackend::new(vec![b"ok-key".to_vec()])),
        );
        coordinator.register_backend(
            AuthMethod::Token,
            Box::new(TokenBackend::new(
                token_signing_key(),
                TokenPolicy {
                    now_unix: 1_700_000_000,
                    expected_audience: Some(String::from("apate-client")),
                    expected_issuer: Some(String::from("apate-auth")),
                },
            )),
        );

        let token = TokenClaims {
            subject: String::from("token-user"),
            expires_at_unix: 1_700_000_300,
            audience: Some(String::from("apate-client")),
            issuer: Some(String::from("apate-auth")),
        }
        .encode_signed(&token_signing_key());
        let identity = coordinator
            .authenticate(AuthInput {
                method: AuthMethod::Token,
                payload: token.into_bytes(),
            })
            .expect("token auth");

        assert_eq!("token-user", identity.subject);
        assert_eq!(AuthMethod::Token, identity.method);
    }
}
