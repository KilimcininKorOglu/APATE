pub mod client_hello;
pub mod entropy;
pub mod facade;
pub mod padding;
pub mod quic_camouflage;
pub mod server_hello;
pub mod timing;
pub mod tls_camouflage;

use crate::config::profiles::{ProfileError, StealthProfile, load_profile};
use std::fs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StealthRuntime {
    profile: StealthProfile,
}

impl StealthRuntime {
    pub fn from_profile_name(
        profile_name: &str,
        profile_override_source: Option<&str>,
    ) -> Result<Self, ProfileError> {
        let profile = load_profile(profile_name, profile_override_source)?;
        Ok(Self { profile })
    }

    pub fn from_profile_path(profile_name: &str, profile_path: &str) -> Result<Self, ProfileError> {
        let override_source = fs::read_to_string(profile_path).map_err(|_| {
            ProfileError::ProfileOverrideReadFailed {
                path: String::from(profile_path),
            }
        })?;
        Self::from_profile_name(profile_name, Some(&override_source))
    }

    pub fn profile(&self) -> &StealthProfile {
        &self.profile
    }
}

#[cfg(test)]
mod tests {
    use crate::config::profiles::CHROME_131;
    use crate::stealth::StealthRuntime;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn runtime_loads_builtin_profile() {
        let runtime = StealthRuntime::from_profile_name(CHROME_131, None).expect("runtime");
        assert_eq!(CHROME_131, runtime.profile().name);
    }

    #[test]
    fn runtime_loads_profile_override_from_file() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("apate_profile_{timestamp}.conf"));
        let override_source = r#"
            name = "file_override"
            alpn = "h3"
            min_packet_size = 920
            max_packet_size = 1200
            min_jitter_ms = 3
            max_jitter_ms = 10
        "#;
        fs::write(&path, override_source).expect("write override");

        let runtime =
            StealthRuntime::from_profile_path("custom", path.to_str().expect("utf8 path"))
                .expect("runtime");
        fs::remove_file(&path).expect("remove override");

        assert_eq!("file_override", runtime.profile().name);
        assert_eq!("h3", runtime.profile().alpn);
    }
}
