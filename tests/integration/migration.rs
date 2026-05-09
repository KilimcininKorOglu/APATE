use apate::transport::connection::TransportEngine;
use apate::transport::mode::ModeNegotiator;
use apate::transport::quic_mask::{QuicMaskConnectPolicy, QuicTransport};
use apate::transport::tcp_tls::{TcpConnectPolicy, TcpTlsTransport};
use apate::transport::udp_tls::{UdpConnectPolicy, UdpTlsTransport};
use apate::transport::{FrameError, TransportError};
use apate::util::{ConnectionState, TransportMode};

fn default_quic() -> QuicTransport {
    QuicTransport::new(QuicMaskConnectPolicy::Success)
}

#[test]
fn rekey_preserves_established_session_traffic_flow() {
    let negotiator = ModeNegotiator::new(TransportMode::Udp, 3);
    let udp = UdpTlsTransport::new(UdpConnectPolicy::Success);
    let tcp = TcpTlsTransport::new(TcpConnectPolicy::Failure);
    let mut engine = TransportEngine::new(negotiator, udp, tcp, default_quic());
    engine.establish().expect("establish");

    let first = engine
        .send_payload(b"before-rekey".to_vec())
        .expect("send 1");
    let epoch = engine.rekey().expect("rekey");
    let second = engine
        .send_payload(b"after-rekey".to_vec())
        .expect("send 2");

    assert_eq!(0, first);
    assert_eq!(1, second);
    assert_eq!(1, epoch);
    assert_eq!(ConnectionState::Established, engine.state());
}

#[test]
fn migration_rejects_invalid_proof_and_accepts_valid_proof() {
    let negotiator = ModeNegotiator::new(TransportMode::Udp, 3);
    let udp = UdpTlsTransport::new(UdpConnectPolicy::Success);
    let tcp = TcpTlsTransport::new(TcpConnectPolicy::Failure);
    let mut engine = TransportEngine::new(negotiator, udp, tcp, default_quic());
    engine.establish().expect("establish");
    let initial_endpoint = String::from(engine.endpoint());
    let next_endpoint = String::from("203.0.113.50:443");

    let invalid_result = engine.migrate_endpoint(next_endpoint.clone(), [0_u8; 16]);
    assert_eq!(
        Err(TransportError::Frame(FrameError::Malformed)),
        invalid_result
    );
    assert_eq!(initial_endpoint, engine.endpoint());

    let valid_proof = engine.migration_proof(&next_endpoint);
    engine
        .migrate_endpoint(next_endpoint.clone(), valid_proof)
        .expect("valid migration");
    let sequence = engine
        .send_payload(b"post-migration".to_vec())
        .expect("send after migration");

    assert_eq!(next_endpoint, engine.endpoint());
    assert_eq!(0, sequence);
    assert_eq!(ConnectionState::Established, engine.state());
}

#[test]
fn migration_proof_is_bound_to_target_endpoint() {
    let negotiator = ModeNegotiator::new(TransportMode::Udp, 3);
    let udp = UdpTlsTransport::new(UdpConnectPolicy::Success);
    let tcp = TcpTlsTransport::new(TcpConnectPolicy::Failure);
    let mut engine = TransportEngine::new(negotiator, udp, tcp, default_quic());
    engine.establish().expect("establish");
    let endpoint_a = String::from("203.0.113.60:443");
    let endpoint_b = String::from("203.0.113.61:443");
    let proof_for_a = engine.migration_proof(&endpoint_a);

    let mismatched = engine.migrate_endpoint(endpoint_b.clone(), proof_for_a);

    assert_eq!(
        Err(TransportError::Frame(FrameError::Malformed)),
        mismatched
    );
    assert_ne!(
        endpoint_b,
        engine.endpoint(),
        "endpoint changed after mismatched migration proof"
    );
}
