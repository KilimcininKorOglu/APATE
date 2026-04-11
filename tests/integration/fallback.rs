use apate::transport::connection::TransportEngine;
use apate::transport::mode::{ModeNegotiator, TransportKind};
use apate::transport::tcp_tls::{TcpConnectPolicy, TcpTlsTransport};
use apate::transport::udp_tls::{UdpConnectPolicy, UdpTlsTransport};
use apate::util::{ConnectionState, TransportMode};

#[test]
fn auto_mode_uses_tcp_after_udp_timeout() {
    let negotiator = ModeNegotiator::new(TransportMode::Auto, 3);
    let udp = UdpTlsTransport::new(UdpConnectPolicy::Timeout);
    let tcp = TcpTlsTransport::new(TcpConnectPolicy::Success);
    let mut engine = TransportEngine::new(negotiator, udp, tcp);

    engine.establish().expect("auto fallback establish");

    assert_eq!(ConnectionState::Established, engine.state());
    assert_eq!(TransportKind::TcpTls, engine.active_kind());
    assert_eq!(1, engine.fallback_count());
}

#[test]
fn forced_tcp_mode_skips_udp_attempt() {
    let negotiator = ModeNegotiator::new(TransportMode::Tcp, 3);
    let udp = UdpTlsTransport::new(UdpConnectPolicy::Timeout);
    let tcp = TcpTlsTransport::new(TcpConnectPolicy::Success);
    let mut engine = TransportEngine::new(negotiator, udp, tcp);

    engine.establish().expect("forced tcp establish");

    assert_eq!(ConnectionState::Established, engine.state());
    assert_eq!(TransportKind::TcpTls, engine.active_kind());
    assert_eq!(0, engine.fallback_count());
}

#[test]
fn forced_quic_mask_mode_uses_quic_transport_path() {
    let negotiator = ModeNegotiator::new(TransportMode::QuicMask, 3);
    let udp = UdpTlsTransport::new(UdpConnectPolicy::Timeout);
    let tcp = TcpTlsTransport::new(TcpConnectPolicy::Failure);
    let mut engine = TransportEngine::new(negotiator, udp, tcp);

    engine.establish().expect("forced quic establish");
    let sequence = engine
        .send_payload(b"integration-quic".to_vec())
        .expect("quic send");

    assert_eq!(ConnectionState::Established, engine.state());
    assert_eq!(TransportKind::QuicMask, engine.active_kind());
    assert_eq!(0, engine.fallback_count());
    assert_eq!(0, sequence);
}
