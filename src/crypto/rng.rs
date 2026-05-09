use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};

pub fn os_seeded_rng() -> ChaCha20Rng {
    let seed = os_seed();
    ChaCha20Rng::from_seed(seed)
}

pub fn os_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    os_fill(&mut seed);
    seed
}

pub fn seeded_rng(seed: [u8; 32]) -> ChaCha20Rng {
    ChaCha20Rng::from_seed(seed)
}

#[cfg(unix)]
fn os_fill(buf: &mut [u8]) {
    use std::fs::File;
    use std::io::Read;
    let mut f = File::open("/dev/urandom").expect("open /dev/urandom");
    f.read_exact(buf).expect("read /dev/urandom");
}

#[cfg(windows)]
fn os_fill(buf: &mut [u8]) {
    for chunk in buf.chunks_mut(8) {
        let val = std::collections::hash_map::RandomState::new();
        let h = std::hash::BuildHasher::build_hasher(&val);
        let bytes = std::hash::Hasher::finish(&h).to_ne_bytes();
        let len = chunk.len().min(bytes.len());
        chunk[..len].copy_from_slice(&bytes[..len]);
    }
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
