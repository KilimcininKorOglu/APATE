pub fn generate_migration_proof(
    session_id: [u8; 16],
    migration_secret: &[u8; 32],
    endpoint: &str,
) -> [u8; 16] {
    let mut payload = Vec::with_capacity(session_id.len() + endpoint.len());
    payload.extend_from_slice(&session_id);
    payload.extend_from_slice(endpoint.as_bytes());
    let digest = blake3::keyed_hash(migration_secret, &payload);

    let mut proof = [0_u8; 16];
    proof.copy_from_slice(&digest.as_bytes()[..16]);
    proof
}

pub fn validate_migration_proof(
    session_id: [u8; 16],
    migration_secret: &[u8; 32],
    endpoint: &str,
    proof: [u8; 16],
) -> bool {
    generate_migration_proof(session_id, migration_secret, endpoint) == proof
}

#[cfg(test)]
mod tests {
    use super::{generate_migration_proof, validate_migration_proof};

    #[test]
    fn migration_proof_roundtrip_for_endpoint() {
        let session_id = [1_u8; 16];
        let secret = [9_u8; 32];
        let endpoint = "198.51.100.10:443";

        let proof = generate_migration_proof(session_id, &secret, endpoint);

        assert!(validate_migration_proof(
            session_id, &secret, endpoint, proof
        ));
    }

    #[test]
    fn migration_proof_rejects_mismatched_endpoint() {
        let session_id = [1_u8; 16];
        let secret = [9_u8; 32];
        let proof = generate_migration_proof(session_id, &secret, "198.51.100.10:443");

        assert!(!validate_migration_proof(
            session_id,
            &secret,
            "198.51.100.11:443",
            proof
        ));
    }
}
