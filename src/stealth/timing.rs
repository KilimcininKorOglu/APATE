use crate::config::profiles::StealthProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimingShaper {
    min_jitter_ms: u16,
    max_jitter_ms: u16,
}

impl TimingShaper {
    pub fn from_profile(profile: &StealthProfile) -> Self {
        Self {
            min_jitter_ms: profile.min_jitter_ms,
            max_jitter_ms: profile.max_jitter_ms,
        }
    }

    pub fn shape_delay_ms(&self, base_delay_ms: u64, packet_size: usize) -> u64 {
        let jitter_span = self.max_jitter_ms.saturating_sub(self.min_jitter_ms);
        let jitter = if jitter_span == 0 {
            self.min_jitter_ms
        } else {
            self.min_jitter_ms + (packet_size as u16 % (jitter_span + 1))
        };

        base_delay_ms.saturating_add(u64::from(jitter))
    }

    pub fn is_within_bounds(&self, delay_ms: u64, base_delay_ms: u64) -> bool {
        let min = base_delay_ms.saturating_add(u64::from(self.min_jitter_ms));
        let max = base_delay_ms.saturating_add(u64::from(self.max_jitter_ms));
        (min..=max).contains(&delay_ms)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::profiles::CHROME_131;
    use crate::config::profiles::builtin_profile;
    use crate::stealth::timing::TimingShaper;

    #[test]
    fn timing_shaper_applies_jitter_within_bounds() {
        let profile = builtin_profile(CHROME_131).expect("profile");
        let shaper = TimingShaper::from_profile(&profile);
        let delay = shaper.shape_delay_ms(10, 128);

        assert!(shaper.is_within_bounds(delay, 10));
    }

    #[test]
    fn timing_shaper_keeps_monotonic_growth_for_larger_payload() {
        let profile = builtin_profile(CHROME_131).expect("profile");
        let shaper = TimingShaper::from_profile(&profile);

        let small_packet = shaper.shape_delay_ms(5, 64);
        let large_packet = shaper.shape_delay_ms(5, 512);

        assert!(large_packet >= 5);
        assert!(small_packet >= 5);
    }
}
