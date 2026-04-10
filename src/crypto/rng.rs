use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};

pub fn seeded_rng(seed: [u8; 32]) -> ChaCha20Rng {
    ChaCha20Rng::from_seed(seed)
}

pub fn fill_random_bytes(rng: &mut ChaCha20Rng, out: &mut [u8]) {
    rng.fill_bytes(out);
}

pub fn next_nonce(rng: &mut ChaCha20Rng) -> [u8; 12] {
    let mut nonce = [0_u8; 12];
    fill_random_bytes(rng, &mut nonce);
    nonce
}

#[cfg(test)]
mod tests {
    use crate::crypto::rng::{next_nonce, seeded_rng};

    #[test]
    fn seeded_rng_produces_deterministic_nonce_stream() {
        let seed = [5_u8; 32];
        let mut first = seeded_rng(seed);
        let mut second = seeded_rng(seed);

        assert_eq!(next_nonce(&mut first), next_nonce(&mut second));
        assert_eq!(next_nonce(&mut first), next_nonce(&mut second));
    }
}
