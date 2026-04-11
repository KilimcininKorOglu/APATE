#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacingScheduler {
    rate_bytes_per_sec: u32,
    next_send_at_ms: u64,
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
    use crate::transport::pacing::PacingScheduler;

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
}
