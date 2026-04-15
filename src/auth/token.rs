use crate::auth::common_crypto::{decode_hex_32, encode_hex, sign_payload};
use crate::auth::{AuthBackend, AuthError, AuthIdentity, AuthInput};
use crate::util::AuthMethod;
use subtle::ConstantTimeEq;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokenPolicy {
    pub now_unix: u64,
    pub expected_audience: Option<String>,
    pub expected_issuer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenClaims {
    pub subject: String,
    pub expires_at_unix: u64,
    pub audience: Option<String>,
    pub issuer: Option<String>,
}

impl TokenClaims {
    pub fn encode_signed(&self, signing_key: &[u8; 32]) -> String {
        let mut unsigned = format!("sub={};exp={}", self.subject, self.expires_at_unix);
        if let Some(audience) = &self.audience {
            unsigned.push_str(&format!(";aud={audience}"));
        }
        if let Some(issuer) = &self.issuer {
            unsigned.push_str(&format!(";iss={issuer}"));
        }

        let signature = sign_payload(signing_key, unsigned.as_bytes());
        format!("{unsigned};sig={}", encode_hex(&signature))
    }
}

#[derive(Debug, Clone)]
pub struct TokenBackend {
    signing_key: [u8; 32],
    policy: TokenPolicy,
}

impl TokenBackend {
    pub fn new(signing_key: [u8; 32], policy: TokenPolicy) -> Self {
        Self {
            signing_key,
            policy,
        }
    }
}

impl AuthBackend for TokenBackend {
    fn authenticate(&self, input: AuthInput) -> Result<AuthIdentity, AuthError> {
        if input.payload.is_empty() {
            return Err(AuthError::EmptyPayload);
        }

        if input.method != AuthMethod::Token {
            return Err(AuthError::UnsupportedBackend {
                method: input.method,
            });
        }

        let token = std::str::from_utf8(&input.payload).map_err(|_| AuthError::Rejected)?;
        let (claims, signature, unsigned_payload) = parse_token(token)?;
        let expected_signature = sign_payload(&self.signing_key, unsigned_payload.as_bytes());
        if signature.ct_eq(&expected_signature).unwrap_u8() != 1 {
            return Err(AuthError::Rejected);
        }

        if claims.expires_at_unix <= self.policy.now_unix {
            return Err(AuthError::Rejected);
        }

        if let Some(expected_audience) = &self.policy.expected_audience
            && claims.audience.as_deref() != Some(expected_audience.as_str())
        {
            return Err(AuthError::Rejected);
        }

        if let Some(expected_issuer) = &self.policy.expected_issuer
            && claims.issuer.as_deref() != Some(expected_issuer.as_str())
        {
            return Err(AuthError::Rejected);
        }

        Ok(AuthIdentity {
            subject: claims.subject,
            method: AuthMethod::Token,
        })
    }
}

fn parse_token(token: &str) -> Result<(TokenClaims, [u8; 32], String), AuthError> {
    let mut subject = None;
    let mut expires_at_unix = None;
    let mut audience = None;
    let mut issuer = None;
    let mut signature = None;
    let mut unsigned_parts = Vec::new();

    for part in token.split(';') {
        if part.is_empty() {
            continue;
        }

        let Some((key, value)) = part.split_once('=') else {
            return Err(AuthError::Rejected);
        };

        match key {
            "sub" => {
                if value.is_empty() {
                    return Err(AuthError::Rejected);
                }
                subject = Some(String::from(value));
                unsigned_parts.push(format!("{key}={value}"));
            }
            "exp" => {
                let parsed = value.parse::<u64>().map_err(|_| AuthError::Rejected)?;
                expires_at_unix = Some(parsed);
                unsigned_parts.push(format!("{key}={value}"));
            }
            "aud" => {
                audience = Some(String::from(value));
                unsigned_parts.push(format!("{key}={value}"));
            }
            "iss" => {
                issuer = Some(String::from(value));
                unsigned_parts.push(format!("{key}={value}"));
            }
            "sig" => {
                signature = Some(decode_hex_32(value)?);
            }
            _ => return Err(AuthError::Rejected),
        }
    }

    let claims = TokenClaims {
        subject: subject.ok_or(AuthError::Rejected)?,
        expires_at_unix: expires_at_unix.ok_or(AuthError::Rejected)?,
        audience,
        issuer,
    };
    let signature = signature.ok_or(AuthError::Rejected)?;
    let unsigned_payload = unsigned_parts.join(";");
    Ok((claims, signature, unsigned_payload))
}

#[cfg(test)]
mod tests {
    use crate::auth::token::{TokenBackend, TokenClaims, TokenPolicy};
    use crate::auth::{AuthBackend, AuthError, AuthInput};
    use crate::util::AuthMethod;

    fn signing_key() -> [u8; 32] {
        [7_u8; 32]
    }

    #[test]
    fn token_backend_accepts_valid_token() {
        let backend = TokenBackend::new(
            signing_key(),
            TokenPolicy {
                now_unix: 1_700_000_000,
                expected_audience: Some(String::from("apate-client")),
                expected_issuer: Some(String::from("apate-auth")),
            },
        );
        let token = TokenClaims {
            subject: String::from("token-user"),
            expires_at_unix: 1_700_000_100,
            audience: Some(String::from("apate-client")),
            issuer: Some(String::from("apate-auth")),
        }
        .encode_signed(&signing_key());

        let identity = backend
            .authenticate(AuthInput {
                method: AuthMethod::Token,
                payload: token.into_bytes(),
            })
            .expect("valid token");

        assert_eq!("token-user", identity.subject);
        assert_eq!(AuthMethod::Token, identity.method);
    }

    #[test]
    fn token_backend_rejects_expired_or_invalid_signature_tokens() {
        let backend = TokenBackend::new(
            signing_key(),
            TokenPolicy {
                now_unix: 1_700_000_000,
                expected_audience: None,
                expected_issuer: None,
            },
        );
        let expired_token = TokenClaims {
            subject: String::from("expired-user"),
            expires_at_unix: 1_699_999_999,
            audience: None,
            issuer: None,
        }
        .encode_signed(&signing_key());
        let invalid_signature = String::from("sub=user;exp=1700000100;sig=deadbeef");

        let expired_error = backend
            .authenticate(AuthInput {
                method: AuthMethod::Token,
                payload: expired_token.into_bytes(),
            })
            .expect_err("expired token");
        let signature_error = backend
            .authenticate(AuthInput {
                method: AuthMethod::Token,
                payload: invalid_signature.into_bytes(),
            })
            .expect_err("invalid signature");

        assert_eq!(AuthError::Rejected, expired_error);
        assert_eq!(AuthError::Rejected, signature_error);
    }
}
