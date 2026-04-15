#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCode {
    Startup,
    RuntimeReady,
    HandshakeSuccess,
    FallbackTriggered,
    LossObserved,
    AuthRejected,
}

impl EventCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::RuntimeReady => "runtime_ready",
            Self::HandshakeSuccess => "handshake_success",
            Self::FallbackTriggered => "fallback_triggered",
            Self::LossObserved => "loss_observed",
            Self::AuthRejected => "auth_rejected",
        }
    }
}

pub fn emit_health_probe(component: &str, healthy: bool, detail: &str) -> String {
    let status = if healthy { "ok" } else { "degraded" };
    format!("health component={component} status={status} detail={detail}")
}

pub fn format_event(code: EventCode, detail: &str) -> String {
    format!("event code={} detail={detail}", code.as_str())
}

#[cfg(test)]
mod tests {
    use crate::telemetry::log::{EventCode, emit_health_probe, format_event};

    #[test]
    fn health_probe_line_uses_expected_shape() {
        let line = emit_health_probe("client_runtime", true, "running");

        assert_eq!(
            "health component=client_runtime status=ok detail=running",
            line
        );
    }

    #[test]
    fn event_line_carries_stable_event_code() {
        let line = format_event(EventCode::FallbackTriggered, "count=1");

        assert_eq!("event code=fallback_triggered detail=count=1", line);
    }
}
