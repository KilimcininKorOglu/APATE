use crate::crypto::CryptoError;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

pub fn encrypt_chacha20poly1305(
    key_bytes: &[u8; 32],
    nonce_bytes: &[u8; 12],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key_bytes));
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_error| CryptoError::EncryptionFailed)
}

pub fn decrypt_chacha20poly1305(
    key_bytes: &[u8; 32],
    nonce_bytes: &[u8; 12],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key_bytes));
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_error| CryptoError::DecryptionFailed)
}

#[cfg(test)]
mod tests {
    use crate::crypto::aead::{decrypt_chacha20poly1305, encrypt_chacha20poly1305};

    #[test]
    fn chacha20poly1305_roundtrip() {
        let key = [7_u8; 32];
        let nonce = [1_u8; 12];
        let aad = b"header";
        let plaintext = b"apate test payload";

        let ciphertext = encrypt_chacha20poly1305(&key, &nonce, plaintext, aad)
            .expect("encryption should succeed");
        let decrypted = decrypt_chacha20poly1305(&key, &nonce, &ciphertext, aad)
            .expect("decryption should succeed");

        assert_eq!(plaintext.to_vec(), decrypted);
    }
}
