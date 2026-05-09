use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Client,
    Server,
    GenKey,
    Version,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliArgs {
    pub command: Command,
    pub config_path: Option<String>,
    pub verbose: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliParseError {
    MissingCommand,
    UnknownCommand(String),
    UnknownFlag(String),
}

impl std::fmt::Display for CliParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCommand => write!(f, "no command specified"),
            Self::UnknownCommand(cmd) => write!(f, "unknown command: {cmd}"),
            Self::UnknownFlag(flag) => write!(f, "unknown flag: {flag}"),
        }
    }
}

impl CliArgs {
    pub fn parse_from_env() -> Result<Self, CliParseError> {
        let args: Vec<String> = env::args().skip(1).collect();
        Self::parse_from_slice(&args)
    }

    pub fn parse_from_slice(args: &[String]) -> Result<Self, CliParseError> {
        let mut command = None;
        let mut config_path = None;
        let mut verbose = false;
        let mut i = 0;

        while i < args.len() {
            let arg = &args[i];
            match arg.as_str() {
                "client" | "server" | "gen-key" | "version" | "help" => {
                    command = Some(match arg.as_str() {
                        "client" => Command::Client,
                        "server" => Command::Server,
                        "gen-key" => Command::GenKey,
                        "version" => Command::Version,
                        _ => Command::Help,
                    });
                }
                "--config" | "-c" => {
                    i += 1;
                    config_path = args.get(i).cloned();
                }
                "--verbose" | "-v" => {
                    verbose = true;
                }
                "--help" | "-h" => {
                    command = Some(Command::Help);
                }
                "--version" | "-V" => {
                    command = Some(Command::Version);
                }
                other if other.starts_with('-') => {
                    return Err(CliParseError::UnknownFlag(String::from(other)));
                }
                other => {
                    return Err(CliParseError::UnknownCommand(String::from(other)));
                }
            }
            i += 1;
        }

        let command = command.ok_or(CliParseError::MissingCommand)?;

        Ok(Self {
            command,
            config_path,
            verbose,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{CliArgs, CliParseError, Command};

    fn args(input: &[&str]) -> Vec<String> {
        input.iter().map(|s| String::from(*s)).collect()
    }

    #[test]
    fn parse_client_command() {
        let parsed = CliArgs::parse_from_slice(&args(&["client"])).expect("parse");
        assert_eq!(Command::Client, parsed.command);
        assert!(!parsed.verbose);
        assert!(parsed.config_path.is_none());
    }

    #[test]
    fn parse_server_with_config() {
        let parsed = CliArgs::parse_from_slice(&args(&["server", "--config", "/etc/apate.conf"]))
            .expect("parse");
        assert_eq!(Command::Server, parsed.command);
        assert_eq!(Some(String::from("/etc/apate.conf")), parsed.config_path);
    }

    #[test]
    fn parse_verbose_flag() {
        let parsed = CliArgs::parse_from_slice(&args(&["-v", "client"])).expect("parse");
        assert!(parsed.verbose);
    }

    #[test]
    fn missing_command_returns_error() {
        let result = CliArgs::parse_from_slice(&args(&[]));
        assert!(matches!(result, Err(CliParseError::MissingCommand)));
    }

    #[test]
    fn unknown_flag_returns_error() {
        let result = CliArgs::parse_from_slice(&args(&["--foobar"]));
        assert!(matches!(result, Err(CliParseError::UnknownFlag(_))));
    }
}
