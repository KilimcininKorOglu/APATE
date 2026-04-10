use crate::crypto::aead::{decrypt_chacha20poly1305, encrypt_chacha20poly1305};
use crate::noise::SecurityError;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CipherState {
    key: [u8; 32],
    nonce: u64,
    has_key: bool,
}

impl CipherState {
    pub fn with_key(key: [u8; 32]) -> Self {
        Self {
            key,
            nonce: 0,
            has_key: true,
        }
    }

    pub fn initialize_key(&mut self, key: [u8; 32]) {
        self.key = key;
        self.nonce = 0;
        self.has_key = true;
    }

    pub fn encrypt(&mut self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, SecurityError> {
        if !self.has_key {
            return Err(SecurityError::InvalidHandshake);
        }

        let nonce_bytes = nonce_to_bytes(self.nonce);
        let ciphertext = encrypt_chacha20poly1305(&self.key, &nonce_bytes, plaintext, aad)
            .map_err(|_error| SecurityError::CipherFailure)?;
        self.nonce = self
            .nonce
            .checked_add(1)
            .ok_or(SecurityError::CipherFailure)?;
        Ok(ciphertext)
    }

    pub fn decrypt(&mut self, ciphertext: &[u8], aad: &[u8]) -> Result<Vec<u8>, SecurityError> {
        if !self.has_key {
            return Err(SecurityError::InvalidHandshake);
        }

        let nonce_bytes = nonce_to_bytes(self.nonce);
        let plaintext = decrypt_chacha20poly1305(&self.key, &nonce_bytes, ciphertext, aad)
            .map_err(|_error| SecurityError::CipherFailure)?;
        self.nonce = self
            .nonce
            .checked_add(1)
            .ok_or(SecurityError::CipherFailure)?;
        Ok(plaintext)
    }
}

fn nonce_to_bytes(nonce: u64) -> [u8; 12] {
    let mut out = [0_u8; 12];
    out[4..].copy_from_slice(&nonce.to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use crate::noise::cipher_state::CipherState;

    #[test]
    fn cipher_state_roundtrip_after_key_init() {
        let mut sender = CipherState::with_key([5_u8; 32]);
        let mut receiver = CipherState::with_key([5_u8; 32]);
        let aad = b"noise";
        let plaintext = b"payload";

        let ciphertext = sender.encrypt(plaintext, aad).expect("encrypt");
        let decrypted = receiver.decrypt(&ciphertext, aad).expect("decrypt");

        assert_eq!(plaintext.to_vec(), decrypted);
    }
}
