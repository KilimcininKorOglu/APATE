use apate::auth::{
    AuthCoordinator, AuthInput, ProbeGatePolicy, ProbeGateResult, StaticKeyBackend,
    evaluate_probe_gate,
};
use apate::stealth::facade::FacadeResponder;
use apate::util::AuthMethod;

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
