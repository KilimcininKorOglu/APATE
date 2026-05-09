use crate::ConfigError;
use crate::config::types::{AppConfig, DnsMode, RoutingMode};
use crate::util::{AuthMethod, TransportMode};
use std::collections::HashMap;

pub fn parse_config(input: &str) -> Result<AppConfig, ConfigError> {
    let mut kv = HashMap::new();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(ConfigError::InvalidValue {
                key: String::from("line_format"),
            });
        };

        let normalized_key = key.trim().to_owned();
        if !is_supported_key(&normalized_key) {
            return Err(ConfigError::UnsupportedKey {
                key: normalized_key,
            });
        }

        let normalized_value = value.trim().to_owned();
        kv.insert(key.trim().to_owned(), normalized_value);
    }

    let mut config = AppConfig::default();

    if let Some(value) = kv.get("client.server") {
        config.client.server = parse_string_value(value);
    }

    if let Some(value) = kv.get("server.listen") {
        config.server.listen = parse_string_value(value);
    }

    if let Some(value) = kv.get("transport.mode") {
        config.transport.mode =
            TransportMode::parse(&parse_string_value(value)).ok_or_else(|| {
                ConfigError::InvalidValue {
                    key: String::from("transport.mode"),
                }
            })?;
    }

    if let Some(value) = kv.get("transport.fallback_timeout") {
        let parsed = parse_string_value(value).parse::<u16>().map_err(|_error| {
            ConfigError::InvalidValue {
                key: String::from("transport.fallback_timeout"),
            }
        })?;
        config.transport.fallback_timeout_secs = parsed;
    }

    if let Some(value) = kv.get("stealth.profile") {
        config.stealth.profile = parse_string_value(value);
    }

    if let Some(value) = kv.get("stealth.profile_path") {
        let parsed = parse_string_value(value);
        config.stealth.profile_path = if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        };
    }

    if let Some(value) = kv.get("stealth.facade_on_auth_failure") {
        config.stealth.facade_on_auth_failure =
            parse_string_value(value)
                .parse::<bool>()
                .map_err(|_error| ConfigError::InvalidValue {
                    key: String::from("stealth.facade_on_auth_failure"),
                })?;
    }

    if let Some(value) = kv.get("auth.methods") {
        config.auth.methods = parse_auth_methods(value)?;
    }

    if let Some(value) = kv.get("crypto.post_quantum") {
        config.crypto.post_quantum =
            parse_string_value(value)
                .parse::<bool>()
                .map_err(|_error| ConfigError::InvalidValue {
                    key: String::from("crypto.post_quantum"),
                })?;
    }

    if let Some(value) = kv.get("crypto.rekey_interval_secs") {
        config.crypto.rekey_interval_secs =
            parse_string_value(value).parse::<u32>().map_err(|_error| {
                ConfigError::InvalidValue {
                    key: String::from("crypto.rekey_interval_secs"),
                }
            })?;
    }

    if let Some(value) = kv.get("crypto.rekey_interval_bytes") {
        config.crypto.rekey_interval_bytes =
            parse_string_value(value).parse::<u64>().map_err(|_error| {
                ConfigError::InvalidValue {
                    key: String::from("crypto.rekey_interval_bytes"),
                }
            })?;
    }

    if let Some(value) = kv.get("routing.mode") {
        config.routing.mode = RoutingMode::parse(&parse_string_value(value)).ok_or_else(|| {
            ConfigError::InvalidValue {
                key: String::from("routing.mode"),
            }
        })?;
    }

    if let Some(value) = kv.get("dns.mode") {
        config.dns.mode = DnsMode::parse(&parse_string_value(value)).ok_or_else(|| {
            ConfigError::InvalidValue {
                key: String::from("dns.mode"),
            }
        })?;
    }

    config.validate()?;
    Ok(config)
}

fn parse_auth_methods(value: &str) -> Result<Vec<AuthMethod>, ConfigError> {
    let raw = parse_string_value(value);
    let normalized = raw.trim();
    if !(normalized.starts_with('[') && normalized.ends_with(']')) {
        return Err(ConfigError::InvalidValue {
            key: String::from("auth.methods"),
        });
    }

    let inner = &normalized[1..normalized.len() - 1];
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut methods = Vec::new();
    for item in inner.split(',') {
        let token = item.trim().trim_matches('"');
        let method = AuthMethod::parse(token).ok_or_else(|| ConfigError::InvalidValue {
            key: String::from("auth.methods"),
        })?;
        methods.push(method);
    }

    Ok(methods)
}

fn parse_string_value(value: &str) -> String {
    value.trim().trim_matches('"').to_owned()
}

fn is_supported_key(key: &str) -> bool {
    matches!(
        key,
        "client.server"
            | "server.listen"
            | "transport.mode"
            | "transport.fallback_timeout"
            | "stealth.profile"
            | "stealth.profile_path"
            | "stealth.facade_on_auth_failure"
            | "auth.methods"
            | "crypto.post_quantum"
            | "crypto.rekey_interval_secs"
            | "crypto.rekey_interval_bytes"
            | "routing.mode"
            | "dns.mode"
    )
}

#[cfg(test)]
mod tests {
    use crate::config::parser::parse_config;
    use crate::util::{AuthMethod, TransportMode};

    #[test]
    fn parse_config_accepts_valid_input() {
        let input = r#"
            client.server = "127.0.0.1:443"
            auth.methods = ["static_key", "token"]
            transport.mode = "auto"
            dns.mode = "doh"
        "#;

        let parsed = parse_config(input).expect("valid config must parse");

        assert_eq!("127.0.0.1:443", parsed.client.server);
        assert_eq!(TransportMode::Auto, parsed.transport.mode);
        assert_eq!(
            vec![AuthMethod::StaticKey, AuthMethod::Token],
            parsed.auth.methods
        );
    }

    #[test]
    fn parse_config_rejects_unsupported_key() {
        let input = r#"
            client.server = "127.0.0.1:443"
            auth.methods = ["static_key"]
            unknown.key = "value"
        "#;

        assert!(parse_config(input).is_err());
    }
}
