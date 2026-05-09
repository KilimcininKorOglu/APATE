use crate::cli::args::{CliArgs, Command};
use crate::config::parser::parse_config;
use crate::config::types::AppConfig;
use crate::telemetry::{EventCode, format_event};
use std::fs;

pub fn dispatch(args: &CliArgs) -> Result<(), String> {
    match args.command {
        Command::Client => run_client(args),
        Command::Server => run_server(args),
        Command::GenKey => run_gen_key(),
        Command::Version => run_version(),
        Command::Help => run_help(),
    }
}

fn load_config(args: &CliArgs) -> Result<AppConfig, String> {
    let config_path = args
        .config_path
        .as_deref()
        .unwrap_or("/etc/apate/apate.conf");

    let source = fs::read_to_string(config_path)
        .map_err(|e| format!("cannot read config {config_path}: {e}"))?;

    let config = parse_config(&source)
        .map_err(|e| format!("config parse error: {e}"))?;

    config
        .validate()
        .map_err(|e| format!("config validation error: {e}"))?;

    Ok(config)
}

fn run_client(args: &CliArgs) -> Result<(), String> {
    let config = load_config(args)?;
    println!(
        "{}",
        format_event(
            EventCode::Startup,
            &format!(
                "mode=client server={} transport={}",
                config.client.server,
                config.transport.mode.as_str()
            )
        )
    );
    Ok(())
}

fn run_server(args: &CliArgs) -> Result<(), String> {
    use crate::auth::{ProbeGatePolicy, ProbeGateResult, evaluate_probe_gate};
    use crate::runtime::Runtime;
    use crate::stealth::facade::FacadeResponder;

    let config = load_config(args)?;
    let methods: Vec<&str> = config.auth.methods.iter().map(|m| m.as_str()).collect();

    let mut runtime = Runtime::new();
    runtime.start().map_err(|e| e.to_string())?;

    let policy = ProbeGatePolicy {
        facade_on_auth_failure: config.stealth.facade_on_auth_failure,
    };
    let facade = FacadeResponder::new(String::from("nginx"));

    println!(
        "{}",
        format_event(
            EventCode::Startup,
            &format!(
                "mode=server auth=[{}] backend={} facade={}",
                methods.join(","),
                runtime.backend_name(),
                policy.facade_on_auth_failure,
            )
        )
    );

    let _ = (policy, facade, evaluate_probe_gate);
    let _ = ProbeGateResult::Reject;
    Ok(())
}

fn run_gen_key() -> Result<(), String> {
    use crate::crypto::kx::derive_public_key;
    use crate::crypto::rng::os_seed;

    let secret = os_seed();
    let public = derive_public_key(secret);
    let hex: String = public.iter().map(|b| format!("{b:02x}")).collect();
    println!("public_key={hex}");
    Ok(())
}

fn run_version() -> Result<(), String> {
    println!("apate {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}

fn run_help() -> Result<(), String> {
    println!(
        "\
apate - stealth VPN tunnel

USAGE:
    apate <COMMAND> [OPTIONS]

COMMANDS:
    client      Start in client mode
    server      Start in server mode
    gen-key     Generate a new X25519 keypair
    version     Print version
    help        Print this help

OPTIONS:
    -c, --config <PATH>    Config file path (default: /etc/apate/apate.conf)
    -v, --verbose          Enable verbose logging
    -h, --help             Print help
    -V, --version          Print version"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::dispatch;
    use crate::cli::args::{CliArgs, Command};

    #[test]
    fn dispatch_version_succeeds() {
        let args = CliArgs {
            command: Command::Version,
            config_path: None,
            verbose: false,
        };
        assert!(dispatch(&args).is_ok());
    }

    #[test]
    fn dispatch_help_succeeds() {
        let args = CliArgs {
            command: Command::Help,
            config_path: None,
            verbose: false,
        };
        assert!(dispatch(&args).is_ok());
    }

    #[test]
    fn dispatch_gen_key_succeeds() {
        let args = CliArgs {
            command: Command::GenKey,
            config_path: None,
            verbose: false,
        };
        assert!(dispatch(&args).is_ok());
    }
}
