use crate::config::profiles::StealthProfile;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaddingShaper {
    min_size: usize,
    max_size: usize,
}

impl PaddingShaper {
    pub fn from_profile(profile: &StealthProfile) -> Self {
        Self {
            min_size: usize::from(profile.min_packet_size),
            max_size: usize::from(profile.max_packet_size),
        }
    }

    pub fn shape(&self, payload: &[u8]) -> Result<Vec<u8>, PaddingError> {
        if payload.len() > self.max_size {
            return Err(PaddingError::PacketTooLarge);
        }

        let mut target_size = payload.len().max(self.min_size);
        if !target_size.is_multiple_of(16) {
            target_size += 16 - (target_size % 16);
        }
        if target_size > self.max_size {
            target_size = self.max_size;
        }
        if target_size < payload.len() {
            return Err(PaddingError::PacketTooLarge);
        }

        let mut shaped = payload.to_vec();
        shaped.resize(target_size, 0);
        Ok(shaped)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PaddingError {
    #[error("payload exceeds profile maximum packet size")]
    PacketTooLarge,
}

#[cfg(test)]
mod tests {
    use crate::config::profiles::CHROME_131;
    use crate::config::profiles::builtin_profile;
    use crate::stealth::padding::{PaddingError, PaddingShaper};

    #[test]
    fn padding_expands_small_packets_to_profile_minimum() {
        let profile = builtin_profile(CHROME_131).expect("profile");
        let shaper = PaddingShaper::from_profile(&profile);
        let shaped = shaper.shape(&[1, 2, 3]).expect("shaped");

        assert!(shaped.len() >= usize::from(profile.min_packet_size));
        assert!(shaped.len() <= usize::from(profile.max_packet_size));
    }

    #[test]
    fn padding_rejects_packets_larger_than_profile_maximum() {
        let profile = builtin_profile(CHROME_131).expect("profile");
        let shaper = PaddingShaper::from_profile(&profile);
        let oversized = vec![0_u8; usize::from(profile.max_packet_size) + 1];

        let shaped = shaper.shape(&oversized);
        assert_eq!(Err(PaddingError::PacketTooLarge), shaped);
    }
}
