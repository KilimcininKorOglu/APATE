use crate::stealth::entropy::EntropySource;

pub struct SessionRotator {
    entropy: EntropySource,
    rotation_interval_ms: u64,
    session_started_at_ms: u64,
    next_rotation_at_ms: u64,
    rotations: u32,
}

impl SessionRotator {
    pub fn new(min_lifetime_secs: u32, max_lifetime_secs: u32) -> Self {
        let mut entropy = EntropySource::new();
        let lifetime_secs = if max_lifetime_secs > min_lifetime_secs {
            min_lifetime_secs
                + entropy.random_in_range(0, (max_lifetime_secs - min_lifetime_secs) as u16) as u32
        } else {
            min_lifetime_secs
        };
        let rotation_interval_ms = u64::from(lifetime_secs) * 1000;

        Self {
            entropy,
            rotation_interval_ms,
            session_started_at_ms: 0,
            next_rotation_at_ms: rotation_interval_ms,
            rotations: 0,
        }
    }

    pub fn start(&mut self, now_ms: u64) {
        self.session_started_at_ms = now_ms;
        self.next_rotation_at_ms = now_ms + self.rotation_interval_ms;
    }

    pub fn should_rotate(&self, now_ms: u64) -> bool {
        now_ms >= self.next_rotation_at_ms
    }

    pub fn on_rotated(&mut self, now_ms: u64, min_lifetime_secs: u32, max_lifetime_secs: u32) {
        self.rotations += 1;
        self.session_started_at_ms = now_ms;

        let lifetime_secs = if max_lifetime_secs > min_lifetime_secs {
            min_lifetime_secs
                + self
                    .entropy
                    .random_in_range(0, (max_lifetime_secs - min_lifetime_secs) as u16)
                    as u32
        } else {
            min_lifetime_secs
        };
        self.rotation_interval_ms = u64::from(lifetime_secs) * 1000;
        self.next_rotation_at_ms = now_ms + self.rotation_interval_ms;
    }

    pub fn rotation_count(&self) -> u32 {
        self.rotations
    }

    pub fn session_age_ms(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.session_started_at_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_triggers_after_lifetime() {
        let mut rotator = SessionRotator::new(1, 1);
        rotator.start(0);
        assert!(!rotator.should_rotate(500));
        assert!(rotator.should_rotate(1000));
    }

    #[test]
    fn rotation_resets_timer() {
        let mut rotator = SessionRotator::new(2, 2);
        rotator.start(0);
        assert!(rotator.should_rotate(2000));

        rotator.on_rotated(2000, 2, 2);
        assert!(!rotator.should_rotate(3000));
        assert!(rotator.should_rotate(4000));
        assert_eq!(1, rotator.rotation_count());
    }

    #[test]
    fn session_age_tracks_current_session() {
        let mut rotator = SessionRotator::new(10, 10);
        rotator.start(1000);
        assert_eq!(500, rotator.session_age_ms(1500));

        rotator.on_rotated(5000, 10, 10);
        assert_eq!(200, rotator.session_age_ms(5200));
    }
}
