use apate::config::parser::parse_config;
use apate::transport::connection::TransportEngine;
use apate::transport::mode::{ModeNegotiator, TransportKind};
use apate::transport::quic_mask::{QuicMaskConnectPolicy, QuicTransport};
use apate::transport::tcp_tls::{TcpConnectPolicy, TcpTlsTransport};
use apate::transport::udp_tls::{UdpConnectPolicy, UdpTlsTransport};
use apate::util::{ConnectionState, TransportMode};

fn default_quic() -> QuicTransport {
    QuicTransport::new(QuicMaskConnectPolicy::Success)
}

#[test]
fn auto_mode_uses_tcp_after_udp_timeout() {
    let negotiator = ModeNegotiator::new(TransportMode::Auto, 3);
    let udp = UdpTlsTransport::new(UdpConnectPolicy::Timeout);
    let tcp = TcpTlsTransport::new(TcpConnectPolicy::Success);
    let mut engine = TransportEngine::new(negotiator, udp, tcp, default_quic());

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
    let mut engine = TransportEngine::new(negotiator, udp, tcp, default_quic());

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
    let mut engine = TransportEngine::new(negotiator, udp, tcp, default_quic());

    engine.establish().expect("forced quic establish");
    let sequence = engine
        .send_payload(b"integration-quic".to_vec())
        .expect("quic send");

    assert_eq!(ConnectionState::Established, engine.state());
    assert_eq!(TransportKind::QuicMask, engine.active_kind());
    assert_eq!(0, engine.fallback_count());
    assert_eq!(0, sequence);
}

#[test]
fn transport_mode_config_permutations_drive_expected_negotiation() {
    let scenarios = vec![
        (
            "auto",
            UdpConnectPolicy::Timeout,
            TcpConnectPolicy::Success,
            TransportKind::TcpTls,
            1_u32,
        ),
        (
            "tcp",
            UdpConnectPolicy::Timeout,
            TcpConnectPolicy::Success,
            TransportKind::TcpTls,
            0_u32,
        ),
        (
            "quic_mask",
            UdpConnectPolicy::Timeout,
            TcpConnectPolicy::Failure,
            TransportKind::QuicMask,
            0_u32,
        ),
    ];

    for (mode_name, udp_policy, tcp_policy, expected_kind, expected_fallbacks) in scenarios {
        let config_source = format!(
            r#"
            client.server = "203.0.113.10:443"
            auth.methods = ["static_key"]
            transport.mode = "{mode_name}"
        "#
        );
        let config = parse_config(&config_source).expect("transport mode permutation parse");
        let negotiator = ModeNegotiator::new(
            config.transport.mode,
            config.transport.fallback_timeout_secs,
        );
        let mut engine = TransportEngine::new(
            negotiator,
            UdpTlsTransport::new(udp_policy),
            TcpTlsTransport::new(tcp_policy),
            default_quic(),
        );
        engine
            .establish()
            .expect("transport mode permutation establish");

        assert_eq!(
            expected_kind,
            engine.active_kind(),
            "mode {mode_name} selected unexpected active transport"
        );
        assert_eq!(
            expected_fallbacks,
            engine.fallback_count(),
            "mode {mode_name} reported unexpected fallback count"
        );
    }
}
