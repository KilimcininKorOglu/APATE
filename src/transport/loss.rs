use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SentPacket {
    pub sequence: u64,
    pub sent_at_ms: u64,
    pub size_bytes: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LossDetector {
    in_flight: BTreeMap<u64, SentPacket>,
}

impl LossDetector {
    pub fn on_packet_sent(&mut self, sequence: u64, sent_at_ms: u64, size_bytes: u16) {
        self.in_flight.insert(
            sequence,
            SentPacket {
                sequence,
                sent_at_ms,
                size_bytes,
            },
        );
    }

    pub fn on_ack_received(&mut self, sequence: u64) {
        self.in_flight.remove(&sequence);
    }

    pub fn detect_lost(&self, now_ms: u64, timeout_ms: u64) -> Vec<u64> {
        self.in_flight
            .values()
            .filter(|packet| now_ms.saturating_sub(packet.sent_at_ms) >= timeout_ms)
            .map(|packet| packet.sequence)
            .collect()
    }

    pub fn in_flight_count(&self) -> usize {
        self.in_flight.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::transport::loss::LossDetector;

    #[test]
    fn loss_detector_flags_expired_packets() {
        let mut detector = LossDetector::default();
        detector.on_packet_sent(1, 100, 1200);
        detector.on_packet_sent(2, 150, 1200);

        let lost = detector.detect_lost(260, 100);

        assert_eq!(vec![1, 2], lost);
    }

    #[test]
    fn acked_packets_leave_in_flight_set() {
        let mut detector = LossDetector::default();
        detector.on_packet_sent(1, 100, 1200);
        detector.on_packet_sent(2, 150, 1200);
        detector.on_ack_received(1);

        let lost = detector.detect_lost(260, 100);

        assert_eq!(vec![2], lost);
        assert_eq!(1, detector.in_flight_count());
    }
}
