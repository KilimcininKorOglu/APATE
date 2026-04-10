use crate::crypto::CryptoError;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

pub fn derive_verifying_key(signing_key_bytes: [u8; 32]) -> [u8; 32] {
    let signing_key = SigningKey::from_bytes(&signing_key_bytes);
    signing_key.verifying_key().to_bytes()
}

pub fn sign_message(signing_key_bytes: [u8; 32], message: &[u8]) -> [u8; 64] {
    let signing_key = SigningKey::from_bytes(&signing_key_bytes);
    signing_key.sign(message).to_bytes()
}

pub fn verify_message(
    verifying_key_bytes: [u8; 32],
    message: &[u8],
    signature_bytes: [u8; 64],
) -> Result<(), CryptoError> {
    let verifying_key = VerifyingKey::from_bytes(&verifying_key_bytes)
        .map_err(|_error| CryptoError::SignatureFailure)?;
    let signature = Signature::from_bytes(&signature_bytes);

    verifying_key
        .verify(message, &signature)
        .map_err(|_error| CryptoError::SignatureFailure)
}

#[cfg(test)]
mod tests {
    use crate::crypto::sign::{derive_verifying_key, sign_message, verify_message};

    #[test]
    fn ed25519_sign_and_verify_roundtrip() {
        let signing_key = [42_u8; 32];
        let verifying_key = derive_verifying_key(signing_key);
        let message = b"apate-signature";
        let signature = sign_message(signing_key, message);

        assert!(verify_message(verifying_key, message, signature).is_ok());
    }
}
