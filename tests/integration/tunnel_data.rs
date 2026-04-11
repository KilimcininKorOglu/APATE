use apate::transport::ack::AckWindow;
use apate::transport::congestion::{CongestionController, CongestionState};
use apate::transport::loss::LossDetector;
use apate::transport::pacing::PacingScheduler;

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
