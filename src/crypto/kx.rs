use crate::crypto::CryptoError;
use x25519_dalek::{PublicKey, StaticSecret};

pub fn derive_public_key(secret_bytes: [u8; 32]) -> [u8; 32] {
    let secret = StaticSecret::from(secret_bytes);
    PublicKey::from(&secret).to_bytes()
}

pub fn derive_shared_secret(
    local_secret_bytes: [u8; 32],
    peer_public_bytes: [u8; 32],
) -> Result<[u8; 32], CryptoError> {
    let local_secret = StaticSecret::from(local_secret_bytes);
    let peer_public = PublicKey::from(peer_public_bytes);
    let shared = local_secret.diffie_hellman(&peer_public);
    Ok(shared.to_bytes())
}

#[cfg(test)]
mod tests {
    use crate::crypto::kx::{derive_public_key, derive_shared_secret};

    #[test]
    fn x25519_shared_secret_matches_both_sides() {
        let alice_secret = [3_u8; 32];
        let bob_secret = [9_u8; 32];

        let alice_public = derive_public_key(alice_secret);
        let bob_public = derive_public_key(bob_secret);

        let shared_a = derive_shared_secret(alice_secret, bob_public).expect("alice shared secret");
        let shared_b = derive_shared_secret(bob_secret, alice_public).expect("bob shared secret");

        assert_eq!(shared_a, shared_b);
    }
}
