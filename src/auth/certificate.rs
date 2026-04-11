use crate::auth::{AuthBackend, AuthError, AuthIdentity, AuthInput};
use crate::util::AuthMethod;
use subtle::ConstantTimeEq;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CertificatePolicy {
    pub now_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustAnchor {
    pub issuer: String,
    pub key: [u8; 32],
}

impl TrustAnchor {
    pub fn parse(source: &str) -> Result<Self, AuthError> {
        let mut issuer = None;
        let mut key = None;

        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let Some((field, value)) = trimmed.split_once('=') else {
                return Err(AuthError::Rejected);
            };

            match field {
                "issuer" => issuer = Some(String::from(value)),
                "key" => key = Some(decode_hex(value)?),
                _ => return Err(AuthError::Rejected),
            }
        }

        Ok(Self {
            issuer: issuer.ok_or(AuthError::Rejected)?,
            key: key.ok_or(AuthError::Rejected)?,
        })
    }

    pub fn from_file(path: &str) -> Result<Self, AuthError> {
        let source = std::fs::read_to_string(path).map_err(|_| AuthError::Internal)?;
        Self::parse(&source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateClaims {
    pub subject: String,
    pub issuer: String,
    pub serial: String,
    pub not_after_unix: u64,
    pub public_key: String,
}

impl CertificateClaims {
    pub fn encode_signed(&self, trust_anchor_key: &[u8; 32]) -> String {
        let unsigned = format!(
            "sub={};iss={};serial={};not_after={};pub={}",
            self.subject, self.issuer, self.serial, self.not_after_unix, self.public_key
        );
        let signature = sign_payload(trust_anchor_key, unsigned.as_bytes());
        format!("{unsigned};sig={}", encode_hex(&signature))
    }
}

#[derive(Debug, Clone)]
pub struct CertificateBackend {
    anchors: Vec<TrustAnchor>,
    policy: CertificatePolicy,
}

impl CertificateBackend {
    pub fn new(anchors: Vec<TrustAnchor>, policy: CertificatePolicy) -> Self {
        Self { anchors, policy }
    }

    pub fn from_anchor_files(paths: &[&str], policy: CertificatePolicy) -> Result<Self, AuthError> {
        let anchors = paths
            .iter()
            .map(|path| TrustAnchor::from_file(path))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::new(anchors, policy))
    }
}

impl AuthBackend for CertificateBackend {
    fn authenticate(&self, input: AuthInput) -> Result<AuthIdentity, AuthError> {
        if input.payload.is_empty() {
            return Err(AuthError::EmptyPayload);
        }

        if input.method != AuthMethod::Certificate {
            return Err(AuthError::UnsupportedBackend {
                method: input.method,
            });
        }

        let certificate = std::str::from_utf8(&input.payload).map_err(|_| AuthError::Rejected)?;
        let (claims, signature, unsigned_payload) = parse_certificate(certificate)?;
        let anchor = self
            .anchors
            .iter()
            .find(|candidate| candidate.issuer == claims.issuer)
            .ok_or(AuthError::Rejected)?;

        let expected_signature = sign_payload(&anchor.key, unsigned_payload.as_bytes());
        if signature.ct_eq(&expected_signature).unwrap_u8() != 1 {
            return Err(AuthError::Rejected);
        }

        if claims.not_after_unix <= self.policy.now_unix {
            return Err(AuthError::Rejected);
        }

        Ok(AuthIdentity {
            subject: claims.subject,
            method: AuthMethod::Certificate,
        })
    }
}

fn parse_certificate(
    certificate: &str,
) -> Result<(CertificateClaims, [u8; 32], String), AuthError> {
    let mut subject = None;
    let mut issuer = None;
    let mut serial = None;
    let mut not_after_unix = None;
    let mut public_key = None;
    let mut signature = None;
    let mut unsigned_parts = Vec::new();

    for part in certificate.split(';') {
        if part.is_empty() {
            continue;
        }

        let Some((field, value)) = part.split_once('=') else {
            return Err(AuthError::Rejected);
        };

        match field {
            "sub" => {
                subject = Some(String::from(value));
                unsigned_parts.push(format!("{field}={value}"));
            }
            "iss" => {
                issuer = Some(String::from(value));
                unsigned_parts.push(format!("{field}={value}"));
            }
            "serial" => {
                serial = Some(String::from(value));
                unsigned_parts.push(format!("{field}={value}"));
            }
            "not_after" => {
                let parsed = value.parse::<u64>().map_err(|_| AuthError::Rejected)?;
                not_after_unix = Some(parsed);
                unsigned_parts.push(format!("{field}={value}"));
            }
            "pub" => {
                public_key = Some(String::from(value));
                unsigned_parts.push(format!("{field}={value}"));
            }
            "sig" => signature = Some(decode_hex(value)?),
            _ => return Err(AuthError::Rejected),
        }
    }

    Ok((
        CertificateClaims {
            subject: subject.ok_or(AuthError::Rejected)?,
            issuer: issuer.ok_or(AuthError::Rejected)?,
            serial: serial.ok_or(AuthError::Rejected)?,
            not_after_unix: not_after_unix.ok_or(AuthError::Rejected)?,
            public_key: public_key.ok_or(AuthError::Rejected)?,
        },
        signature.ok_or(AuthError::Rejected)?,
        unsigned_parts.join(";"),
    ))
}

fn sign_payload(signing_key: &[u8; 32], payload: &[u8]) -> [u8; 32] {
    *blake3::keyed_hash(signing_key, payload).as_bytes()
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(value: &str) -> Result<[u8; 32], AuthError> {
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

#[cfg(test)]
mod tests {
    use crate::auth::certificate::{
        CertificateBackend, CertificateClaims, CertificatePolicy, TrustAnchor,
    };
    use crate::auth::{AuthBackend, AuthError, AuthInput};
    use crate::util::AuthMethod;

    fn trust_anchor() -> TrustAnchor {
        TrustAnchor {
            issuer: String::from("apate-test-ca"),
            key: [0x11_u8; 32],
        }
    }

    #[test]
    fn trust_anchor_parser_accepts_valid_source() {
        let parsed = TrustAnchor::parse(
            "issuer=apate-test-ca\nkey=1111111111111111111111111111111111111111111111111111111111111111",
        )
        .expect("anchor parse");

        assert_eq!(trust_anchor(), parsed);
    }

    #[test]
    fn certificate_backend_accepts_trusted_certificate() {
        let backend = CertificateBackend::new(
            vec![trust_anchor()],
            CertificatePolicy {
                now_unix: 1_700_000_000,
            },
        );
        let certificate = CertificateClaims {
            subject: String::from("cert-user"),
            issuer: String::from("apate-test-ca"),
            serial: String::from("SER-001"),
            not_after_unix: 1_700_000_100,
            public_key: String::from("pk-001"),
        }
        .encode_signed(&[0x11_u8; 32]);

        let identity = backend
            .authenticate(AuthInput {
                method: AuthMethod::Certificate,
                payload: certificate.into_bytes(),
            })
            .expect("trusted certificate");

        assert_eq!("cert-user", identity.subject);
        assert_eq!(AuthMethod::Certificate, identity.method);
    }

    #[test]
    fn certificate_backend_rejects_expired_or_untrusted_certificate() {
        let backend = CertificateBackend::new(
            vec![trust_anchor()],
            CertificatePolicy {
                now_unix: 1_700_000_000,
            },
        );
        let expired = CertificateClaims {
            subject: String::from("expired-cert"),
            issuer: String::from("apate-test-ca"),
            serial: String::from("SER-002"),
            not_after_unix: 1_699_999_900,
            public_key: String::from("pk-002"),
        }
        .encode_signed(&[0x11_u8; 32]);
        let untrusted = CertificateClaims {
            subject: String::from("untrusted-cert"),
            issuer: String::from("unknown-ca"),
            serial: String::from("SER-003"),
            not_after_unix: 1_700_000_100,
            public_key: String::from("pk-003"),
        }
        .encode_signed(&[0x22_u8; 32]);

        let expired_error = backend
            .authenticate(AuthInput {
                method: AuthMethod::Certificate,
                payload: expired.into_bytes(),
            })
            .expect_err("expired cert");
        let untrusted_error = backend
            .authenticate(AuthInput {
                method: AuthMethod::Certificate,
                payload: untrusted.into_bytes(),
            })
            .expect_err("untrusted cert");

        assert_eq!(AuthError::Rejected, expired_error);
        assert_eq!(AuthError::Rejected, untrusted_error);
    }
}
