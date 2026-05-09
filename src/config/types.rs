use crate::ConfigError;
use crate::util::{AuthMethod, TransportMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingMode {
    Full,
    Split,
}

impl RoutingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Split => "split",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "full" => Some(Self::Full),
            "split" => Some(Self::Split),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsMode {
    Doh,
    Plain,
    Fallback,
}

impl DnsMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Doh => "doh",
            Self::Plain => "plain",
            Self::Fallback => "fallback",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "doh" => Some(Self::Doh),
            "plain" => Some(Self::Plain),
            "fallback" => Some(Self::Fallback),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClientConfig {
    pub server: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    pub listen: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: String::from("0.0.0.0:443"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportConfig {
    pub mode: TransportMode,
    pub fallback_timeout_secs: u16,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            mode: TransportMode::Auto,
            fallback_timeout_secs: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StealthConfig {
    pub profile: String,
    pub profile_path: Option<String>,
    pub facade_on_auth_failure: bool,
}

impl Default for StealthConfig {
    fn default() -> Self {
        Self {
            profile: String::from("chrome_131"),
            profile_path: None,
            facade_on_auth_failure: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuthConfig {
    pub methods: Vec<AuthMethod>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoConfig {
    pub post_quantum: bool,
    pub rekey_interval_secs: u32,
    pub rekey_interval_bytes: u64,
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            post_quantum: true,
            rekey_interval_secs: 60,
            rekey_interval_bytes: 1_073_741_824,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingConfig {
    pub mode: RoutingMode,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            mode: RoutingMode::Full,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsConfig {
    pub mode: DnsMode,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self { mode: DnsMode::Doh }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppConfig {
    pub client: ClientConfig,
    pub server: ServerConfig,
    pub transport: TransportConfig,
    pub stealth: StealthConfig,
    pub auth: AuthConfig,
    pub crypto: CryptoConfig,
    pub routing: RoutingConfig,
    pub dns: DnsConfig,
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.transport.fallback_timeout_secs == 0 {
            return Err(ConfigError::InvalidValue {
                key: String::from("transport.fallback_timeout"),
            });
        }

        Ok(())
    }

    pub fn validate_client(&self) -> Result<(), ConfigError> {
        self.validate()?;

        if self.client.server.trim().is_empty() {
            return Err(ConfigError::MissingRequiredKey {
                key: String::from("client.server"),
            });
        }

        Ok(())
    }

    pub fn validate_server(&self) -> Result<(), ConfigError> {
        self.validate()?;

        if self.server.listen.trim().is_empty() {
            return Err(ConfigError::MissingRequiredKey {
                key: String::from("server.listen"),
            });
        }

        if self.auth.methods.is_empty() {
            return Err(ConfigError::MissingRequiredKey {
                key: String::from("auth.methods"),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::config::types::{AppConfig, AuthConfig, ClientConfig, DnsMode, RoutingMode};
    use crate::util::{AuthMethod, TransportMode};

    #[test]
    fn routing_mode_roundtrip_parse() {
        let mode = RoutingMode::Split;
        let as_str = mode.as_str();

        assert_eq!(Some(mode), RoutingMode::parse(as_str));
    }

    #[test]
    fn dns_mode_roundtrip_parse() {
        let mode = DnsMode::Fallback;
        let as_str = mode.as_str();

        assert_eq!(Some(mode), DnsMode::parse(as_str));
    }

    #[test]
    fn client_validation_requires_server_endpoint() {
        let config = AppConfig {
            client: ClientConfig {
                server: String::new(),
            },
            ..Default::default()
        };

        assert!(config.validate_client().is_err());
    }

    #[test]
    fn client_validation_passes_with_server_endpoint() {
        let config = AppConfig {
            client: ClientConfig {
                server: String::from("1.2.3.4:443"),
            },
            ..Default::default()
        };

        assert!(config.validate_client().is_ok());
    }

    #[test]
    fn server_validation_requires_auth_methods() {
        let config = AppConfig {
            auth: AuthConfig {
                methods: Vec::new(),
            },
            ..Default::default()
        };

        assert!(config.validate_server().is_err());
    }

    #[test]
    fn server_validation_passes_with_auth_methods() {
        let config = AppConfig {
            auth: AuthConfig {
                methods: vec![AuthMethod::StaticKey],
            },
            ..Default::default()
        };

        assert!(config.validate_server().is_ok());
    }

    #[test]
    fn base_validation_rejects_zero_timeout() {
        let mut config = AppConfig::default();
        config.transport.fallback_timeout_secs = 0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn defaults_use_expected_values() {
        let config = AppConfig::default();

        assert_eq!(TransportMode::Auto, config.transport.mode);
        assert!(config.stealth.facade_on_auth_failure);
        assert_eq!("0.0.0.0:443", config.server.listen);
        assert_eq!("chrome_131", config.stealth.profile);
    }
}
