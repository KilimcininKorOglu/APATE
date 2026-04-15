#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MetricsRegistry {
    handshake_success: u64,
    fallback_count: u64,
    loss_events: u64,
    auth_rejections: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MetricsSnapshot {
    pub handshake_success: u64,
    pub fallback_count: u64,
    pub loss_events: u64,
    pub auth_rejections: u64,
}

impl MetricsRegistry {
    pub fn record_handshake_success(&mut self) {
        self.handshake_success = self.handshake_success.saturating_add(1);
    }

    pub fn record_fallback(&mut self) {
        self.fallback_count = self.fallback_count.saturating_add(1);
    }

    pub fn record_loss_event(&mut self) {
        self.loss_events = self.loss_events.saturating_add(1);
    }

    pub fn record_auth_rejection(&mut self) {
        self.auth_rejections = self.auth_rejections.saturating_add(1);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            handshake_success: self.handshake_success,
            fallback_count: self.fallback_count,
            loss_events: self.loss_events,
            auth_rejections: self.auth_rejections,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::telemetry::metrics::{MetricsRegistry, MetricsSnapshot};

    #[test]
    fn registry_tracks_all_required_counters() {
        let mut registry = MetricsRegistry::default();
        registry.record_handshake_success();
        registry.record_fallback();
        registry.record_loss_event();
        registry.record_auth_rejection();

        assert_eq!(
            MetricsSnapshot {
                handshake_success: 1,
                fallback_count: 1,
                loss_events: 1,
                auth_rejections: 1,
            },
            registry.snapshot()
        );
    }
}
