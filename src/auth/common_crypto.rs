use crate::auth::AuthError;

pub(super) fn sign_payload(signing_key: &[u8; 32], payload: &[u8]) -> [u8; 32] {
    *blake3::keyed_hash(signing_key, payload).as_bytes()
}

pub(super) fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn decode_hex_32(value: &str) -> Result<[u8; 32], AuthError> {
    if value.len() != 64 {
        return Err(AuthError::Rejected);
    }

    let mut output = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = from_hex(chunk[0])?;
        let low = from_hex(chunk[1])?;
        output[index] = (high << 4) | low;
    }

    Ok(output)
}

fn from_hex(byte: u8) -> Result<u8, AuthError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(AuthError::Rejected),
    }
}
