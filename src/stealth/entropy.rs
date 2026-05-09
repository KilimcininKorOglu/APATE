use crate::crypto::rng::{fill_random_bytes, os_seeded_rng};
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::Rng;

pub struct EntropySource {
    rng: ChaCha20Rng,
}

impl EntropySource {
    pub fn new() -> Self {
        Self {
            rng: os_seeded_rng(),
        }
    }

    pub fn fill_padding(&mut self, buf: &mut [u8]) {
        fill_random_bytes(&mut self.rng, buf);
    }

    pub fn random_in_range(&mut self, min: u16, max: u16) -> u16 {
        if min >= max {
            return min;
        }
        let range = u32::from(max - min) + 1;
        let raw = self.rng.next_u32();
        min + (raw % range) as u16
    }

    pub fn random_delay_us(&mut self, min_us: u64, max_us: u64) -> u64 {
        if min_us >= max_us {
            return min_us;
        }
        let range = max_us - min_us + 1;
        let raw = self.rng.next_u64();
        min_us + (raw % range)
    }
}

impl Default for EntropySource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::EntropySource;

    #[test]
    fn fill_padding_produces_nonzero_output() {
        let mut src = EntropySource::new();
        let mut buf = [0u8; 64];
        src.fill_padding(&mut buf);
        assert!(buf.iter().any(|&b| b != 0));
    }

    #[test]
    fn random_in_range_stays_within_bounds() {
        let mut src = EntropySource::new();
        for _ in 0..200 {
            let val = src.random_in_range(10, 50);
            assert!((10..=50).contains(&val));
        }
    }

    #[test]
    fn random_in_range_equal_bounds_returns_min() {
        let mut src = EntropySource::new();
        assert_eq!(42, src.random_in_range(42, 42));
    }

    #[test]
    fn random_delay_us_stays_within_bounds() {
        let mut src = EntropySource::new();
        for _ in 0..200 {
            let delay = src.random_delay_us(100, 5000);
            assert!((100..=5000).contains(&delay));
        }
    }
}
