use crate::util::AuthMethod;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthInput {
    pub method: AuthMethod,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthIdentity {
    pub subject: String,
    pub method: AuthMethod,
}

pub trait AuthBackend {
    fn authenticate(&self, input: AuthInput) -> Result<AuthIdentity, AuthError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AuthError {
    #[error("auth payload is empty")]
    EmptyPayload,
    #[error("auth backend unsupported for method: {method:?}")]
    UnsupportedBackend { method: AuthMethod },
    #[error("authentication rejected")]
    Rejected,
    #[error("internal auth failure")]
    Internal,
}

#[cfg(test)]
mod tests {
    use crate::auth::AuthError;
    use crate::util::AuthMethod;

    #[test]
    fn auth_error_message_stable() {
        let error = AuthError::UnsupportedBackend {
            method: AuthMethod::Token,
        };

        assert_eq!(
            "auth backend unsupported for method: Token",
            error.to_string()
        );
    }
}
