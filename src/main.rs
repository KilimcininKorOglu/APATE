use apate::cli::args::CliArgs;
use apate::cli::commands::dispatch;
use apate::telemetry::{EventCode, format_event};

fn main() {
    let args = match CliArgs::parse_from_env() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            eprintln!("run 'apate help' for usage");
            std::process::exit(2);
        }
    };

    if let Err(error) = dispatch(&args) {
        eprintln!(
            "{}",
            format_event(EventCode::Startup, &format!("state=error reason={error}"))
        );
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use apate::cli::args::{CliArgs, Command};
    use apate::cli::commands::dispatch;

    #[test]
    fn dispatch_version_produces_output() {
        let args = CliArgs {
            command: Command::Version,
            config_path: None,
            verbose: false,
        };
        assert!(dispatch(&args).is_ok());
    }

    #[test]
    fn dispatch_help_produces_output() {
        let args = CliArgs {
            command: Command::Help,
            config_path: None,
            verbose: false,
        };
        assert!(dispatch(&args).is_ok());
    }
}
