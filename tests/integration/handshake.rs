use apate::auth::{
    AuthBackend, AuthError, AuthIdentity, AuthInput, ProbeGatePolicy, ProbeGateResult,
    evaluate_probe_gate,
};
use apate::stealth::facade::FacadeResponder;
use apate::util::AuthMethod;

#[derive(Debug, Clone)]
struct StaticTestBackend {
    key: Vec<u8>,
}

impl StaticTestBackend {
    fn new(key: Vec<u8>) -> Self {
        Self { key }
    }
}

impl AuthBackend for StaticTestBackend {
    fn authenticate(&self, input: AuthInput) -> Result<AuthIdentity, AuthError> {
        if input.payload.is_empty() {
            return Err(AuthError::EmptyPayload);
        }

        if input.method != AuthMethod::StaticKey {
            return Err(AuthError::UnsupportedBackend {
                method: input.method,
            });
        }

        if input.payload == self.key {
            Ok(AuthIdentity {
                subject: String::from("static-client"),
                method: AuthMethod::StaticKey,
            })
        } else {
            Err(AuthError::Rejected)
        }
    }
}

#[test]
fn valid_auth_routes_to_tunnel_path() {
    let backend = StaticTestBackend::new(b"top-secret".to_vec());
    let gate = evaluate_probe_gate(
        backend.authenticate(AuthInput {
            method: AuthMethod::StaticKey,
            payload: b"top-secret".to_vec(),
        }),
        ProbeGatePolicy {
            facade_on_auth_failure: true,
        },
    );

    let ProbeGateResult::AllowTunnel(identity) = gate else {
        panic!("expected authenticated tunnel admission");
    };

    assert_eq!("static-client", identity.subject);
    assert_eq!(AuthMethod::StaticKey, identity.method);
}

#[test]
fn invalid_auth_routes_to_facade_without_tunnel_leaks() {
    let backend = StaticTestBackend::new(b"top-secret".to_vec());
    let gate = evaluate_probe_gate(
        backend.authenticate(AuthInput {
            method: AuthMethod::StaticKey,
            payload: b"wrong".to_vec(),
        }),
        ProbeGatePolicy {
            facade_on_auth_failure: true,
        },
    );

    assert_eq!(ProbeGateResult::ServeFacade, gate);

    let facade = FacadeResponder::new(String::from("nginx"));
    let response = facade.respond_for_probe("/health");

    assert_eq!(200, response.status_code);
    assert_eq!("text/html; charset=utf-8", response.content_type);
    assert!(!response.body.contains("tunnel"));
    assert!(!response.body.contains("rejected"));
}
