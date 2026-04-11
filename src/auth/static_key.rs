use crate::auth::{AuthBackend, AuthError, AuthIdentity, AuthInput};
use crate::util::AuthMethod;
use subtle::ConstantTimeEq;

#[derive(Debug, Clone)]
pub struct StaticKeyBackend {
    keys: Vec<Vec<u8>>,
}

impl StaticKeyBackend {
    pub fn new(keys: Vec<Vec<u8>>) -> Self {
        Self { keys }
    }
}

impl AuthBackend for StaticKeyBackend {
    fn authenticate(&self, input: AuthInput) -> Result<AuthIdentity, AuthError> {
        if input.payload.is_empty() {
            return Err(AuthError::EmptyPayload);
        }

        if input.method != AuthMethod::StaticKey {
            return Err(AuthError::UnsupportedBackend {
                method: input.method,
            });
        }

        if self.keys.iter().any(|key| key.ct_eq(&input.payload).into()) {
            return Ok(AuthIdentity {
                subject: String::from("static-key"),
                method: AuthMethod::StaticKey,
            });
        }

        Err(AuthError::Rejected)
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::static_key::StaticKeyBackend;
    use crate::auth::{AuthBackend, AuthError, AuthInput};
    use crate::util::AuthMethod;

    #[test]
    fn static_key_backend_accepts_matching_key() {
        let backend = StaticKeyBackend::new(vec![b"key-a".to_vec()]);
        let identity = backend
            .authenticate(AuthInput {
                method: AuthMethod::StaticKey,
                payload: b"key-a".to_vec(),
            })
            .expect("static key accepted");

        assert_eq!("static-key", identity.subject);
        assert_eq!(AuthMethod::StaticKey, identity.method);
    }

    #[test]
    fn static_key_backend_rejects_unknown_key() {
        let backend = StaticKeyBackend::new(vec![b"key-a".to_vec()]);
        let error = backend
            .authenticate(AuthInput {
                method: AuthMethod::StaticKey,
                payload: b"key-b".to_vec(),
            })
            .expect_err("unknown key");

        assert_eq!(AuthError::Rejected, error);
    }
}
