use crate::config::profiles::StealthProfile;
use crate::stealth::entropy::EntropySource;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaddingShaper {
    min_size: usize,
    max_size: usize,
    download_pad_pct: u8,
    upload_pad_pct: u8,
}

impl PaddingShaper {
    pub fn from_profile(profile: &StealthProfile) -> Self {
        Self {
            min_size: usize::from(profile.min_packet_size),
            max_size: usize::from(profile.max_packet_size),
            download_pad_pct: 60,
            upload_pad_pct: 10,
        }
    }

    pub fn shape(&self, payload: &[u8]) -> Result<Vec<u8>, PaddingError> {
        self.shape_directed(payload, PacketDirection::Upload)
    }

    pub fn shape_directed(
        &self,
        payload: &[u8],
        direction: PacketDirection,
    ) -> Result<Vec<u8>, PaddingError> {
        if payload.len() > self.max_size {
            return Err(PaddingError::PacketTooLarge);
        }

        let mut target_size = payload.len().max(self.min_size);
        if !target_size.is_multiple_of(16) {
            target_size += 16 - (target_size % 16);
        }

        let extra_pct = match direction {
            PacketDirection::Download => self.download_pad_pct,
            PacketDirection::Upload => self.upload_pad_pct,
        };
        if extra_pct > 0 {
            let extra = target_size * usize::from(extra_pct) / 100;
            target_size += extra;
            if !target_size.is_multiple_of(16) {
                target_size += 16 - (target_size % 16);
            }
        }

        if target_size > self.max_size {
            target_size = self.max_size;
        }
        if target_size < payload.len() {
            return Err(PaddingError::PacketTooLarge);
        }

        let mut shaped = payload.to_vec();
        if shaped.len() < target_size {
            let pad_start = shaped.len();
            shaped.resize(target_size, 0);
            let mut entropy = EntropySource::new();
            entropy.fill_padding(&mut shaped[pad_start..]);
        }
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
    use crate::stealth::padding::{PacketDirection, PaddingError, PaddingShaper};

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

    #[test]
    fn download_padding_exceeds_upload_padding() {
        let profile = builtin_profile(CHROME_131).expect("profile");
        let shaper = PaddingShaper::from_profile(&profile);
        let payload = vec![0x42; 200];

        let upload = shaper
            .shape_directed(&payload, PacketDirection::Upload)
            .expect("upload");
        let download = shaper
            .shape_directed(&payload, PacketDirection::Download)
            .expect("download");

        assert!(
            download.len() >= upload.len(),
            "download {} should be >= upload {}",
            download.len(),
            upload.len(),
        );
    }

    #[test]
    fn padding_bytes_are_not_all_zero() {
        let profile = builtin_profile(CHROME_131).expect("profile");
        let shaper = PaddingShaper::from_profile(&profile);
        let payload = vec![0x00; 10];

        let shaped = shaper
            .shape_directed(&payload, PacketDirection::Download)
            .expect("shaped");
        let pad_region = &shaped[10..];
        let nonzero = pad_region.iter().any(|&b| b != 0);
        assert!(
            nonzero,
            "padding should contain random bytes, not all zeros"
        );
    }
}
