mod chrome_131;
mod firefox_130;
mod safari_18;

use thiserror::Error;

pub const CHROME_131: &str = "chrome_131";
pub const FIREFOX_130: &str = "firefox_130";
pub const SAFARI_18: &str = "safari_18";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StealthProfile {
    pub name: String,
    pub alpn: String,
    pub min_packet_size: u16,
    pub max_packet_size: u16,
    pub min_jitter_ms: u16,
    pub max_jitter_ms: u16,
}

impl StealthProfile {
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.name.trim().is_empty() || self.name.len() > 64 {
            return Err(ProfileError::InvalidProfile {
                field: String::from("name"),
            });
        }
        if self.alpn.trim().is_empty() || self.alpn.len() > 16 {
            return Err(ProfileError::InvalidProfile {
                field: String::from("alpn"),
            });
        }
        if !(64..=4096).contains(&self.min_packet_size) {
            return Err(ProfileError::InvalidProfile {
                field: String::from("min_packet_size"),
            });
        }
        if self.max_packet_size < self.min_packet_size || self.max_packet_size > 4096 {
            return Err(ProfileError::InvalidProfile {
                field: String::from("max_packet_size"),
            });
        }
        if self.max_jitter_ms < self.min_jitter_ms || self.max_jitter_ms > 500 {
            return Err(ProfileError::InvalidProfile {
                field: String::from("max_jitter_ms"),
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProfileError {
    #[error("unknown built-in stealth profile: {name}")]
    UnknownBuiltinProfile { name: String },
    #[error("failed to read profile override from path: {path}")]
    ProfileOverrideReadFailed { path: String },
    #[error("invalid profile field: {field}")]
    InvalidProfile { field: String },
    #[error("invalid override key: {key}")]
    InvalidOverrideKey { key: String },
}

pub fn is_builtin_profile(name: &str) -> bool {
    builtin_profile(name).is_some()
}

pub fn builtin_profile(name: &str) -> Option<StealthProfile> {
    match name {
        CHROME_131 => Some(chrome_131::profile()),
        FIREFOX_130 => Some(firefox_130::profile()),
        SAFARI_18 => Some(safari_18::profile()),
        _ => None,
    }
}

pub fn load_profile(
    name: &str,
    override_content: Option<&str>,
) -> Result<StealthProfile, ProfileError> {
    let profile = match override_content {
        Some(override_source) => parse_override(name, override_source)?,
        None => builtin_profile(name).ok_or_else(|| ProfileError::UnknownBuiltinProfile {
            name: String::from(name),
        })?,
    };

    profile.validate()?;
    Ok(profile)
}

fn parse_override(name: &str, source: &str) -> Result<StealthProfile, ProfileError> {
    let mut profile = builtin_profile(name).unwrap_or(StealthProfile {
        name: String::from(name),
        alpn: String::from("h2"),
        min_packet_size: 768,
        max_packet_size: 1280,
        min_jitter_ms: 2,
        max_jitter_ms: 20,
    });

    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(ProfileError::InvalidProfile {
                field: String::from("override_line_format"),
            });
        };

        let key = key.trim();
        let value = value.trim().trim_matches('"');
        match key {
            "name" => profile.name = String::from(value),
            "alpn" => profile.alpn = String::from(value),
            "min_packet_size" => {
                profile.min_packet_size =
                    value
                        .parse::<u16>()
                        .map_err(|_| ProfileError::InvalidProfile {
                            field: String::from("min_packet_size"),
                        })?;
            }
            "max_packet_size" => {
                profile.max_packet_size =
                    value
                        .parse::<u16>()
                        .map_err(|_| ProfileError::InvalidProfile {
                            field: String::from("max_packet_size"),
                        })?;
            }
            "min_jitter_ms" => {
                profile.min_jitter_ms =
                    value
                        .parse::<u16>()
                        .map_err(|_| ProfileError::InvalidProfile {
                            field: String::from("min_jitter_ms"),
                        })?;
            }
            "max_jitter_ms" => {
                profile.max_jitter_ms =
                    value
                        .parse::<u16>()
                        .map_err(|_| ProfileError::InvalidProfile {
                            field: String::from("max_jitter_ms"),
                        })?;
            }
            _ => {
                return Err(ProfileError::InvalidOverrideKey {
                    key: String::from(key),
                });
            }
        }
    }

    Ok(profile)
}

#[cfg(test)]
mod tests {
    use crate::config::profiles::{
        CHROME_131, FIREFOX_130, ProfileError, SAFARI_18, builtin_profile, is_builtin_profile,
        load_profile,
    };

    #[test]
    fn chrome_profile_exists() {
        assert!(is_builtin_profile(CHROME_131));
    }

    #[test]
    fn firefox_profile_exists() {
        assert!(builtin_profile(FIREFOX_130).is_some());
    }

    #[test]
    fn safari_profile_exists() {
        assert!(builtin_profile(SAFARI_18).is_some());
    }

    #[test]
    fn custom_profile_override_is_validated() {
        let override_source = r#"
            name = "custom"
            alpn = "h3"
            min_packet_size = 900
            max_packet_size = 1100
            min_jitter_ms = 2
            max_jitter_ms = 8
        "#;

        let loaded = load_profile("custom", Some(override_source)).expect("profile");
        assert_eq!("custom", loaded.name);
        assert_eq!("h3", loaded.alpn);
    }

    #[test]
    fn invalid_override_key_is_rejected() {
        let override_source = r#"
            invalid_key = "1"
        "#;

        let loaded = load_profile("custom", Some(override_source));
        assert_eq!(
            Err(ProfileError::InvalidOverrideKey {
                key: String::from("invalid_key")
            }),
            loaded
        );
    }
}
