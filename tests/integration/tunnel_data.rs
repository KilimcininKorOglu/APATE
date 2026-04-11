use apate::transport::FecMode;
use apate::transport::ack::AckWindow;
use apate::transport::congestion::{CongestionController, CongestionState};
use apate::transport::connection::TransportEngine;
use apate::transport::loss::LossDetector;
use apate::transport::mode::ModeNegotiator;
use apate::transport::pacing::PacingScheduler;
use apate::transport::tcp_tls::{TcpConnectPolicy, TcpTlsTransport};
use apate::transport::udp_tls::{UdpConnectPolicy, UdpTlsTransport};
use apate::tunnel::{LinuxTunAdapter, MacOsTunAdapter, TunnelAdapter, TunnelPacket};
use apate::util::TransportMode;
use apate::{
    config::parser::parse_config,
    config::types::{DnsMode, RoutingMode},
    routing::{Cidr, DnsAction, DohForwarder, RouteTarget, RoutingEngine},
};

#[test]
fn retransmit_flow_recovers_dropped_packet() {
    let mut ack_window = AckWindow::default();
    let mut loss_detector = LossDetector::default();

    loss_detector.on_packet_sent(1, 0, 1200);
    loss_detector.on_packet_sent(2, 20, 1200);
    ack_window.observe(2);
    loss_detector.on_ack_received(2);

    let lost = loss_detector.detect_lost(200, 100);
    assert_eq!(vec![1], lost);

    loss_detector.on_packet_sent(1, 210, 1200);
    ack_window.observe(1);
    loss_detector.on_ack_received(1);

    assert!(ack_window.is_acked(1));
    assert!(loss_detector.detect_lost(260, 100).is_empty());
}

#[test]
fn pacing_delay_remains_bounded_under_load() {
    let mut scheduler = PacingScheduler::new(2_400);
    let mut last_send_at = 0_u64;

    for _ in 0..100 {
        last_send_at = scheduler.schedule(0, 1_200);
    }

    assert!(last_send_at <= 60_000);
    assert!(scheduler.next_send_at_ms() <= 60_500);
}

#[test]
fn congestion_controller_transitions_after_loss_and_ack() {
    let mut controller = CongestionController::new(9_600, 9_600, 1_200);
    controller.on_loss();
    assert_eq!(CongestionState::Recovery, controller.state());

    controller.on_ack();
    assert_eq!(CongestionState::CongestionAvoidance, controller.state());
}

#[test]
fn linux_tunnel_adapter_exchanges_packet_in_loopback_path() {
    let raw = [
        0x45, 0x00, 0x00, 0x14, 0, 0, 0, 0, 64, 6, 0, 0, 10, 0, 0, 1, 10, 0, 0, 2,
    ];
    let packet = TunnelPacket::parse(&raw).expect("valid ipv4 packet");
    let mut adapter = LinuxTunAdapter::new(String::from("tun1"));
    adapter.open().expect("linux open");
    adapter.configure(1500).expect("linux configure");

    adapter
        .write_packet(packet.clone())
        .expect("linux write packet");
    let received = adapter
        .read_packet()
        .expect("linux read result")
        .expect("linux packet expected");

    assert_eq!(packet.as_bytes(), received.as_bytes());
}

#[test]
fn macos_tunnel_adapter_exchanges_packet_in_loopback_path() {
    let mut raw = vec![0_u8; 40];
    raw[0] = 0x60;
    let packet = TunnelPacket::parse(&raw).expect("valid ipv6 packet");
    let mut adapter = MacOsTunAdapter::new(String::from("utun5"));
    adapter.open().expect("macos open");
    adapter.configure(1500).expect("macos configure");

    adapter
        .write_packet(packet.clone())
        .expect("macos write packet");
    let received = adapter
        .read_packet()
        .expect("macos read result")
        .expect("macos packet expected");

    assert_eq!(packet.as_bytes(), received.as_bytes());
}

#[test]
fn split_routing_prefers_more_specific_prefix_for_tunnel_path() {
    let mut engine = RoutingEngine::new(RoutingMode::Split, DnsMode::Doh, true);
    engine.add_split_tunnel_route(Cidr::parse("10.0.0.0/8").expect("cidr"));
    engine.add_split_tunnel_route(Cidr::parse("10.10.0.0/16").expect("cidr"));

    let tunneled = engine.route_packet([10, 10, 1, 7].into());
    let bypassed = engine.route_packet([192, 0, 2, 10].into());

    assert_eq!(RouteTarget::Tunnel, tunneled);
    assert_eq!(RouteTarget::Bypass, bypassed);
}

#[test]
fn dns_fallback_blocks_bypass_when_doh_unavailable() {
    let engine = RoutingEngine::new(RoutingMode::Split, DnsMode::Fallback, false);
    let bypass_destination = [1, 1, 1, 1].into();
    let action = engine.route_dns_query(bypass_destination);

    assert_eq!(DnsAction::Blocked, action);
}

#[test]
fn doh_forwarder_builds_query_for_dual_stack_records() {
    let forwarder =
        DohForwarder::new(String::from("https://resolver.example/dns-query")).expect("forwarder");
    let a_query = forwarder.build_query("example.com", 1, 0x1200).expect("a");
    let aaaa_query = forwarder
        .build_query("example.com", 28, 0x1201)
        .expect("aaaa");

    assert!(a_query.len() > 12);
    assert!(aaaa_query.len() > 12);
}

#[test]
fn adaptive_fec_enables_parity_under_loss_and_disables_in_tcp_mode() {
    let negotiator = ModeNegotiator::new(TransportMode::Udp, 3);
    let udp = UdpTlsTransport::new(UdpConnectPolicy::Success);
    let tcp = TcpTlsTransport::new(TcpConnectPolicy::Failure);
    let mut udp_engine = TransportEngine::new(negotiator, udp, tcp);
    udp_engine.establish().expect("udp establish");
    udp_engine.update_observed_loss(0.22);

    let parity = udp_engine.build_fec_parity(&[vec![1_u8, 2, 3], vec![4_u8, 5, 6]]);

    assert_eq!(FecMode::DoubleParity, udp_engine.fec_mode());
    assert_eq!(2, parity.len());

    let tcp_negotiator = ModeNegotiator::new(TransportMode::Tcp, 3);
    let tcp_udp = UdpTlsTransport::new(UdpConnectPolicy::Timeout);
    let tcp_transport = TcpTlsTransport::new(TcpConnectPolicy::Success);
    let mut tcp_engine = TransportEngine::new(tcp_negotiator, tcp_udp, tcp_transport);
    tcp_engine.establish().expect("tcp establish");
    tcp_engine.update_observed_loss(0.30);

    assert_eq!(FecMode::Disabled, tcp_engine.fec_mode());
    assert!(
        tcp_engine
            .build_fec_parity(&[vec![9_u8, 9], vec![8_u8, 8]])
            .is_empty()
    );
}

#[test]
fn routing_and_dns_config_permutations_route_expected_paths() {
    let scenarios = vec![
        (
            "split",
            "fallback",
            false,
            RouteTarget::Bypass,
            DnsAction::Blocked,
        ),
        (
            "full",
            "plain",
            true,
            RouteTarget::Tunnel,
            DnsAction::UsePlain,
        ),
    ];

    for (routing_mode, dns_mode, doh_available, expected_route, expected_dns) in scenarios {
        let config_source = format!(
            r#"
            client.server = "203.0.113.20:443"
            auth.methods = ["static_key"]
            routing.mode = "{routing_mode}"
            dns.mode = "{dns_mode}"
        "#
        );
        let config = parse_config(&config_source).expect("routing/dns permutation parse");
        let mut engine = RoutingEngine::new(config.routing.mode, config.dns.mode, doh_available);
        engine.add_split_tunnel_route(Cidr::parse("10.0.0.0/8").expect("cidr"));

        let route_target = engine.route_packet([198, 51, 100, 5].into());
        let dns_action = engine.route_dns_query([198, 51, 100, 5].into());

        assert_eq!(
            expected_route, route_target,
            "routing.mode={routing_mode} produced unexpected route"
        );
        assert_eq!(
            expected_dns, dns_action,
            "dns.mode={dns_mode} produced unexpected dns policy"
        );
    }
}
