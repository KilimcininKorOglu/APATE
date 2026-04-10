use crate::crypto::CryptoError;
use zeroize::Zeroize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretKeyMaterial {
    bytes: [u8; 32],
}

impl SecretKeyMaterial {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    pub fn erase(&mut self) {
        self.bytes.zeroize();
    }
}

pub fn derive_key_material(
    input_key_material: &[u8],
    salt: &[u8],
    context: &[u8],
) -> Result<SecretKeyMaterial, CryptoError> {
    if input_key_material.is_empty() {
        return Err(CryptoError::InvalidKeyLength);
    }

    let mut hasher = blake3::Hasher::new();
    hasher.update(salt);
    hasher.update(input_key_material);
    hasher.update(context);

    let mut out = [0_u8; 32];
    out.copy_from_slice(hasher.finalize().as_bytes());
    Ok(SecretKeyMaterial::from_bytes(out))
}

#[cfg(test)]
mod tests {
    use crate::crypto::kdf::{SecretKeyMaterial, derive_key_material};

    #[test]
    fn kdf_is_deterministic() {
        let first = derive_key_material(b"ikm", b"salt", b"context").expect("first derive");
        let second = derive_key_material(b"ikm", b"salt", b"context").expect("second derive");

        assert_eq!(first, second);
    }

    #[test]
    fn key_material_erase_zeroizes_bytes() {
        let mut material = SecretKeyMaterial::from_bytes([9_u8; 32]);
        material.erase();

        assert_eq!([0_u8; 32], *material.as_bytes());
    }
}
