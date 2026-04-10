use crate::crypto::kdf::derive_key_material;
use crate::noise::SecurityError;
use crate::noise::cipher_state::CipherState;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SymmetricState {
    pub chaining_key: [u8; 32],
    pub handshake_hash: [u8; 32],
    pub cipher_state: CipherState,
}

impl SymmetricState {
    pub fn mix_key(&mut self, input_key_material: &[u8]) -> Result<(), SecurityError> {
        let material =
            derive_key_material(input_key_material, &self.chaining_key, b"noise-mix-key")
                .map_err(|_error| SecurityError::KeyDerivationFailed)?;
        self.chaining_key = *material.as_bytes();
        self.cipher_state.initialize_key(self.chaining_key);
        Ok(())
    }

    pub fn mix_hash(&mut self, data: &[u8]) {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.handshake_hash);
        hasher.update(data);
        self.handshake_hash
            .copy_from_slice(hasher.finalize().as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use crate::noise::symmetric_state::SymmetricState;

    #[test]
    fn mix_key_initializes_cipher_state() {
        let mut state = SymmetricState::default();
        let plaintext = b"hello";
        let aad = b"aad";

        state.mix_key(b"input").expect("mix key");
        let ciphertext = state
            .cipher_state
            .encrypt(plaintext, aad)
            .expect("encrypt after mix_key");

        assert!(!ciphertext.is_empty());
    }
}
