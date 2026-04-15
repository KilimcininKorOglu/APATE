pub mod log;
pub mod metrics;

pub use log::{EventCode, emit_health_probe, format_event};
pub use metrics::{MetricsRegistry, MetricsSnapshot};

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryVerbosity {
    Minimal,
    Normal,
    Debug,
}

impl TelemetryVerbosity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Normal => "normal",
            Self::Debug => "debug",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryConfig {
    pub verbosity: TelemetryVerbosity,
    pub emit_metrics: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            verbosity: TelemetryVerbosity::Minimal,
            emit_metrics: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TelemetryError {
    #[error("telemetry sink unavailable")]
    SinkUnavailable,
    #[error("telemetry buffer full")]
    BufferFull,
}

#[cfg(test)]
mod tests {
    use crate::telemetry::{TelemetryConfig, TelemetryVerbosity};

    #[test]
    fn default_telemetry_is_minimal() {
        let config = TelemetryConfig::default();

        assert_eq!(TelemetryVerbosity::Minimal, config.verbosity);
        assert_eq!("minimal", config.verbosity.as_str());
    }
}
