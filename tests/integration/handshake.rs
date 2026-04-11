use apate::auth::{
    AuthCoordinator, AuthInput, CertificateBackend, CertificateClaims, CertificatePolicy,
    ProbeGatePolicy, ProbeGateResult, StaticKeyBackend, TokenBackend, TokenClaims, TokenPolicy,
    evaluate_probe_gate,
};
use apate::config::parser::parse_config;
use apate::config::profiles::FIREFOX_130;
use apate::stealth::StealthRuntime;
use apate::stealth::facade::FacadeResponder;
use apate::util::AuthMethod;

fn token_signing_key() -> [u8; 32] {
    [9_u8; 32]
}

fn fixture_path(name: &str) -> String {
    format!("{}/tests/fixtures/certs/{name}", env!("CARGO_MANIFEST_DIR"))
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

#[test]
fn valid_certificate_auth_routes_to_tunnel_when_ca_is_trusted() {
    let mut coordinator = AuthCoordinator::new(vec![AuthMethod::Certificate]);
    let cert_backend = CertificateBackend::from_anchor_files(
        &[fixture_path("ca_primary.anchor").as_str()],
        CertificatePolicy {
            now_unix: 1_700_000_000,
        },
    )
    .expect("load trusted ca");
    coordinator.register_backend(AuthMethod::Certificate, Box::new(cert_backend));

    let certificate = CertificateClaims {
        subject: String::from("cert-user"),
        issuer: String::from("apate-test-ca"),
        serial: String::from("CERT-INT-001"),
        not_after_unix: 1_700_000_500,
        public_key: String::from("pk-cert"),
    }
    .encode_signed(&[0x11_u8; 32]);
    let gate = evaluate_probe_gate(
        coordinator.authenticate(AuthInput {
            method: AuthMethod::Certificate,
            payload: certificate.into_bytes(),
        }),
        ProbeGatePolicy {
            facade_on_auth_failure: true,
        },
    );

    let ProbeGateResult::AllowTunnel(identity) = gate else {
        panic!("expected certificate-authenticated tunnel admission");
    };
    assert_eq!("cert-user", identity.subject);
    assert_eq!(AuthMethod::Certificate, identity.method);
}

#[test]
fn untrusted_certificate_routes_to_facade() {
    let mut coordinator = AuthCoordinator::new(vec![AuthMethod::Certificate]);
    let cert_backend = CertificateBackend::from_anchor_files(
        &[fixture_path("ca_primary.anchor").as_str()],
        CertificatePolicy {
            now_unix: 1_700_000_000,
        },
    )
    .expect("load trusted ca");
    coordinator.register_backend(AuthMethod::Certificate, Box::new(cert_backend));

    let untrusted_certificate = CertificateClaims {
        subject: String::from("untrusted-user"),
        issuer: String::from("apate-secondary-ca"),
        serial: String::from("CERT-INT-002"),
        not_after_unix: 1_700_000_500,
        public_key: String::from("pk-untrusted"),
    }
    .encode_signed(&[0x22_u8; 32]);
    let gate = evaluate_probe_gate(
        coordinator.authenticate(AuthInput {
            method: AuthMethod::Certificate,
            payload: untrusted_certificate.into_bytes(),
        }),
        ProbeGatePolicy {
            facade_on_auth_failure: true,
        },
    );

    assert_eq!(ProbeGateResult::ServeFacade, gate);
}

#[test]
fn config_selected_profile_and_auth_permutation_loads_runtime() {
    let config_source = r#"
        client.server = "198.51.100.10:443"
        auth.methods = ["static_key", "certificate"]
        transport.mode = "quic_mask"
        stealth.profile = "firefox_130"
        routing.mode = "split"
        dns.mode = "fallback"
    "#;
    let config = parse_config(config_source).expect("config permutation must parse");
    let runtime = StealthRuntime::from_profile_name(&config.stealth.profile, None)
        .expect("selected profile must load");

    assert_eq!(
        FIREFOX_130,
        runtime.profile().name,
        "profile selection mismatch for handshake permutation"
    );
    assert_eq!(
        vec![AuthMethod::StaticKey, AuthMethod::Certificate],
        config.auth.methods,
        "auth method order mismatch in parsed permutation"
    );
}
