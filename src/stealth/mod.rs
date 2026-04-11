use crate::config::profiles::{ProfileError, StealthProfile, load_profile};

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

    pub fn profile(&self) -> &StealthProfile {
        &self.profile
    }
}

#[cfg(test)]
mod tests {
    use crate::config::profiles::CHROME_131;
    use crate::stealth::StealthRuntime;

    #[test]
    fn runtime_loads_builtin_profile() {
        let runtime = StealthRuntime::from_profile_name(CHROME_131, None).expect("runtime");
        assert_eq!(CHROME_131, runtime.profile().name);
    }
}
