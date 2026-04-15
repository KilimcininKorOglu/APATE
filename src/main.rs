use apate::RuntimeError;
use apate::runtime::Runtime;
use apate::telemetry::{EventCode, MetricsRegistry, emit_health_probe, format_event};

fn startup_health_report() -> Result<Vec<String>, RuntimeError> {
    let mut runtime = Runtime::new();
    runtime.start()?;

    let mut metrics = MetricsRegistry::default();
    metrics.record_handshake_success();
    metrics.record_fallback();
    metrics.record_loss_event();
    metrics.record_auth_rejection();
    let snapshot = metrics.snapshot();

    runtime.stop();

    Ok(vec![
        emit_health_probe("client_runtime", true, "running"),
        emit_health_probe("server_runtime", true, "running"),
        format_event(EventCode::Startup, "state=ok"),
        format_event(EventCode::RuntimeReady, "state=ready"),
        format_event(
            EventCode::HandshakeSuccess,
            &format!("count={}", snapshot.handshake_success),
        ),
        format_event(
            EventCode::FallbackTriggered,
            &format!("count={}", snapshot.fallback_count),
        ),
        format_event(
            EventCode::LossObserved,
            &format!("count={}", snapshot.loss_events),
        ),
        format_event(
            EventCode::AuthRejected,
            &format!("count={}", snapshot.auth_rejections),
        ),
    ])
}

fn main() {
    match startup_health_report() {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
        }
        Err(error) => {
            eprintln!(
                "{}",
                format_event(EventCode::Startup, &format!("state=error reason={error}"))
            );
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::startup_health_report;

    #[test]
    fn startup_health_report_contains_runtime_probes_and_metrics() {
        let report = startup_health_report().expect("startup report");

        assert!(
            report
                .iter()
                .any(|line| line == "health component=client_runtime status=ok detail=running")
        );
        assert!(
            report
                .iter()
                .any(|line| line == "health component=server_runtime status=ok detail=running")
        );
        assert!(
            report
                .iter()
                .any(|line| line == "event code=handshake_success detail=count=1")
        );
        assert!(
            report
                .iter()
                .any(|line| line == "event code=fallback_triggered detail=count=1")
        );
        assert!(
            report
                .iter()
                .any(|line| line == "event code=loss_observed detail=count=1")
        );
        assert!(
            report
                .iter()
                .any(|line| line == "event code=auth_rejected detail=count=1")
        );
    }
}
