pub mod aead;
pub mod kdf;
pub mod kx;
pub mod rng;
pub mod sign;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CryptoError {
    #[error("invalid key length")]
    InvalidKeyLength,
    #[error("encryption failed")]
    EncryptionFailed,
    #[error("decryption failed")]
    DecryptionFailed,
    #[error("rng failure")]
    RngFailure,
    #[error("key exchange failure")]
    KxFailure,
    #[error("signature verification failure")]
    SignatureFailure,
}
