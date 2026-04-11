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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeGatePolicy {
    pub facade_on_auth_failure: bool,
}

impl Default for ProbeGatePolicy {
    fn default() -> Self {
        Self {
            facade_on_auth_failure: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeGateResult {
    AllowTunnel(AuthIdentity),
    ServeFacade,
    Reject,
}

pub fn evaluate_probe_gate(
    auth_result: Result<AuthIdentity, AuthError>,
    policy: ProbeGatePolicy,
) -> ProbeGateResult {
    match auth_result {
        Ok(identity) => ProbeGateResult::AllowTunnel(identity),
        Err(_) if policy.facade_on_auth_failure => ProbeGateResult::ServeFacade,
        Err(_) => ProbeGateResult::Reject,
    }
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
    use crate::auth::{
        AuthError, AuthIdentity, ProbeGatePolicy, ProbeGateResult, evaluate_probe_gate,
    };
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

    #[test]
    fn probe_gate_routes_failed_auth_to_facade_when_enabled() {
        let result = evaluate_probe_gate(
            Err(AuthError::Rejected),
            ProbeGatePolicy {
                facade_on_auth_failure: true,
            },
        );

        assert_eq!(ProbeGateResult::ServeFacade, result);
    }

    #[test]
    fn probe_gate_allows_authenticated_identity() {
        let result = evaluate_probe_gate(
            Ok(AuthIdentity {
                subject: String::from("client"),
                method: AuthMethod::StaticKey,
            }),
            ProbeGatePolicy::default(),
        );

        assert!(matches!(result, ProbeGateResult::AllowTunnel(_)));
    }
}
