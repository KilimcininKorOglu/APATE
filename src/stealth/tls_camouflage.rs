use crate::config::profiles::StealthProfile;
use crate::stealth::client_hello::ClientHelloSpec;
use crate::stealth::server_hello::ServerHelloSpec;
use thiserror::Error;

const TLS_VERSION_1_2: u16 = 0x0303;
const MAX_TLS_RECORD_PAYLOAD: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsRecordType {
    Handshake = 22,
    ApplicationData = 23,
}

impl TlsRecordType {
    fn from_byte(byte: u8) -> Result<Self, TlsCamouflageError> {
        match byte {
            22 => Ok(Self::Handshake),
            23 => Ok(Self::ApplicationData),
            _ => Err(TlsCamouflageError::InvalidRecordType),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsRecord {
    pub record_type: TlsRecordType,
    pub version: u16,
    pub payload: Vec<u8>,
}

impl TlsRecord {
    pub fn new(record_type: TlsRecordType, payload: Vec<u8>) -> Result<Self, TlsCamouflageError> {
        if payload.len() > MAX_TLS_RECORD_PAYLOAD {
            return Err(TlsCamouflageError::RecordTooLarge);
        }

        Ok(Self {
            record_type,
            version: TLS_VERSION_1_2,
            payload,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, TlsCamouflageError> {
        if self.payload.len() > MAX_TLS_RECORD_PAYLOAD {
            return Err(TlsCamouflageError::RecordTooLarge);
        }

        let payload_len =
            u16::try_from(self.payload.len()).map_err(|_| TlsCamouflageError::RecordTooLarge)?;
        let mut encoded = Vec::with_capacity(5 + self.payload.len());
        encoded.push(self.record_type as u8);
        encoded.extend_from_slice(&self.version.to_be_bytes());
        encoded.extend_from_slice(&payload_len.to_be_bytes());
        encoded.extend_from_slice(&self.payload);

        Ok(encoded)
    }

    pub fn decode(input: &[u8]) -> Result<Self, TlsCamouflageError> {
        if input.len() < 5 {
            return Err(TlsCamouflageError::MalformedRecord);
        }

        let record_type = TlsRecordType::from_byte(input[0])?;
        let version = u16::from_be_bytes([input[1], input[2]]);
        let payload_len = u16::from_be_bytes([input[3], input[4]]) as usize;
        if payload_len > MAX_TLS_RECORD_PAYLOAD || input.len() != payload_len + 5 {
            return Err(TlsCamouflageError::MalformedRecord);
        }

        Ok(Self {
            record_type,
            version,
            payload: input[5..].to_vec(),
        })
    }
}

pub fn build_client_hello_record(
    profile: &StealthProfile,
) -> Result<TlsRecord, TlsCamouflageError> {
    let hello = ClientHelloSpec::from_profile(profile);
    let payload = hello.encode()?;
    TlsRecord::new(TlsRecordType::Handshake, payload)
}

pub fn build_server_hello_record(
    alpn: String,
    selected_cipher: u16,
    selected_extension: u16,
) -> Result<TlsRecord, TlsCamouflageError> {
    let hello = ServerHelloSpec::new(alpn, selected_cipher, selected_extension);
    let payload = hello.encode()?;
    TlsRecord::new(TlsRecordType::Handshake, payload)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TlsCamouflageError {
    #[error("invalid tls record type")]
    InvalidRecordType,
    #[error("tls record payload exceeds supported size")]
    RecordTooLarge,
    #[error("malformed tls record")]
    MalformedRecord,
    #[error("invalid hello field")]
    InvalidHelloField,
}

#[cfg(test)]
mod tests {
    use crate::config::profiles::CHROME_131;
    use crate::config::profiles::builtin_profile;
    use crate::stealth::tls_camouflage::{TlsRecord, build_client_hello_record};

    #[test]
    fn tls_record_roundtrip_preserves_payload() {
        let profile = builtin_profile(CHROME_131).expect("profile");
        let record = build_client_hello_record(&profile).expect("record");
        let encoded = record.encode().expect("encoded");
        let decoded = TlsRecord::decode(&encoded).expect("decoded");

        assert_eq!(record.payload, decoded.payload);
    }

    #[test]
    fn tls_record_rejects_invalid_type() {
        let invalid = [0x01, 0x03, 0x03, 0x00, 0x00];
        let decoded = TlsRecord::decode(&invalid);
        assert!(decoded.is_err());
    }
}
