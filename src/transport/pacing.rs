#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacingScheduler {
    rate_bytes_per_sec: u32,
    next_send_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressionPolicy {
    min_payload_bytes: usize,
    max_unique_ratio: f32,
}

impl Default for CompressionPolicy {
    fn default() -> Self {
        Self {
            min_payload_bytes: 64,
            max_unique_ratio: 0.45,
        }
    }
}

impl CompressionPolicy {
    pub fn should_compress(&self, payload: &[u8]) -> bool {
        if payload.len() < self.min_payload_bytes {
            return false;
        }

        let mut seen = [false; 256];
        for byte in payload {
            seen[usize::from(*byte)] = true;
        }
        let unique_count = seen.iter().filter(|flag| **flag).count();
        let ratio = unique_count as f32 / payload.len() as f32;
        ratio <= self.max_unique_ratio
    }

    pub fn compress_rle(&self, payload: &[u8]) -> Option<Vec<u8>> {
        if !self.should_compress(payload) {
            return None;
        }

        let mut compressed = Vec::new();
        let mut index = 0;
        while index < payload.len() {
            let byte = payload[index];
            let mut run_len = 1usize;
            while index + run_len < payload.len()
                && payload[index + run_len] == byte
                && run_len < u8::MAX as usize
            {
                run_len += 1;
            }

            compressed.push(run_len as u8);
            compressed.push(byte);
            index += run_len;
        }

        if compressed.len() < payload.len() {
            Some(compressed)
        } else {
            None
        }
    }
}

impl PacingScheduler {
    pub fn new(rate_bytes_per_sec: u32) -> Self {
        Self {
            rate_bytes_per_sec: rate_bytes_per_sec.max(1),
            next_send_at_ms: 0,
        }
    }

    pub fn next_send_at_ms(&self) -> u64 {
        self.next_send_at_ms
    }

    pub fn schedule(&mut self, now_ms: u64, packet_bytes: u16) -> u64 {
        let start_at = now_ms.max(self.next_send_at_ms);
        let pacing_delay_ms =
            (u64::from(packet_bytes) * 1_000).div_ceil(u64::from(self.rate_bytes_per_sec));
        self.next_send_at_ms = start_at.saturating_add(pacing_delay_ms);
        start_at
    }
}

#[cfg(test)]
mod tests {
    use crate::transport::pacing::{CompressionPolicy, PacingScheduler};

    #[test]
    fn pacing_scheduler_keeps_monotonic_send_times() {
        let mut scheduler = PacingScheduler::new(1_200);

        let first = scheduler.schedule(0, 1_200);
        let second = scheduler.schedule(0, 1_200);

        assert_eq!(0, first);
        assert!(second > first);
        assert!(scheduler.next_send_at_ms() > second);
    }

    #[test]
    fn pacing_scheduler_uses_current_time_after_idle_gap() {
        let mut scheduler = PacingScheduler::new(1_000);
        scheduler.schedule(0, 1_000);

        let send_at = scheduler.schedule(5_000, 1_000);

        assert_eq!(5_000, send_at);
    }

    #[test]
    fn compression_policy_prefers_low_entropy_payloads() {
        let policy = CompressionPolicy::default();
        let low_entropy = vec![7_u8; 128];
        let high_entropy = (0..128).map(|value| value as u8).collect::<Vec<_>>();

        assert!(policy.should_compress(&low_entropy));
        assert!(!policy.should_compress(&high_entropy));
    }

    #[test]
    fn compression_policy_emits_smaller_rle_payload() {
        let policy = CompressionPolicy::default();
        let payload = vec![1_u8; 80];
        let compressed = policy.compress_rle(&payload).expect("rle compression");

        assert!(compressed.len() < payload.len());
    }
}
