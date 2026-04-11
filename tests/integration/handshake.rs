use apate::auth::{
    AuthCoordinator, AuthInput, ProbeGatePolicy, ProbeGateResult, StaticKeyBackend, TokenBackend,
    TokenClaims, TokenPolicy, evaluate_probe_gate,
};
use apate::stealth::facade::FacadeResponder;
use apate::util::AuthMethod;

fn token_signing_key() -> [u8; 32] {
    [9_u8; 32]
}

#[test]
fn valid_auth_routes_to_tunnel_path() {
    let mut coordinator = AuthCoordinator::new(vec![AuthMethod::StaticKey]);
    coordinator.register_backend(
        AuthMethod::StaticKey,
        Box::new(StaticKeyBackend::new(vec![b"top-secret".to_vec()])),
    );
    let gate = evaluate_probe_gate(
        coordinator.authenticate(AuthInput {
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

    assert_eq!("static-key", identity.subject);
    assert_eq!(AuthMethod::StaticKey, identity.method);
}

#[test]
fn invalid_auth_routes_to_facade_without_tunnel_leaks() {
    let mut coordinator = AuthCoordinator::new(vec![AuthMethod::StaticKey]);
    coordinator.register_backend(
        AuthMethod::StaticKey,
        Box::new(StaticKeyBackend::new(vec![b"top-secret".to_vec()])),
    );
    let gate = evaluate_probe_gate(
        coordinator.authenticate(AuthInput {
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

#[test]
fn valid_token_auth_routes_to_tunnel_in_mixed_backend_config() {
    let mut coordinator = AuthCoordinator::new(vec![AuthMethod::StaticKey, AuthMethod::Token]);
    coordinator.register_backend(
        AuthMethod::StaticKey,
        Box::new(StaticKeyBackend::new(vec![b"top-secret".to_vec()])),
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
        expires_at_unix: 1_700_000_100,
        audience: Some(String::from("apate-client")),
        issuer: Some(String::from("apate-auth")),
    }
    .encode_signed(&token_signing_key());

    let gate = evaluate_probe_gate(
        coordinator.authenticate(AuthInput {
            method: AuthMethod::Token,
            payload: token.into_bytes(),
        }),
        ProbeGatePolicy {
            facade_on_auth_failure: true,
        },
    );

    let ProbeGateResult::AllowTunnel(identity) = gate else {
        panic!("expected token-authenticated tunnel admission");
    };
    assert_eq!("token-user", identity.subject);
    assert_eq!(AuthMethod::Token, identity.method);
}

#[test]
fn invalid_token_routes_to_facade_in_mixed_backend_config() {
    let mut coordinator = AuthCoordinator::new(vec![AuthMethod::StaticKey, AuthMethod::Token]);
    coordinator.register_backend(
        AuthMethod::StaticKey,
        Box::new(StaticKeyBackend::new(vec![b"top-secret".to_vec()])),
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
    let expired_token = TokenClaims {
        subject: String::from("token-user"),
        expires_at_unix: 1_699_999_900,
        audience: Some(String::from("apate-client")),
        issuer: Some(String::from("apate-auth")),
    }
    .encode_signed(&token_signing_key());

    let gate = evaluate_probe_gate(
        coordinator.authenticate(AuthInput {
            method: AuthMethod::Token,
            payload: expired_token.into_bytes(),
        }),
        ProbeGatePolicy {
            facade_on_auth_failure: true,
        },
    );

    assert_eq!(ProbeGateResult::ServeFacade, gate);
}
