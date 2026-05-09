use crate::config::profiles::StealthProfile;
use crate::stealth::tls_camouflage::TlsCamouflageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientHelloSpec {
    pub alpn: String,
    pub session_id: [u8; 32],
    pub cipher_suites: Vec<u16>,
    pub extensions: Vec<u16>,
}

impl ClientHelloSpec {
    pub fn from_profile(profile: &StealthProfile) -> Self {
        let mut session_id = [0_u8; 32];
        for (index, byte) in profile.name.bytes().take(32).enumerate() {
            session_id[index] = byte;
        }

        Self {
            alpn: profile.alpn.clone(),
            session_id,
            cipher_suites: profile.cipher_suites.clone(),
            extensions: profile.extensions.clone(),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, TlsCamouflageError> {
        if self.alpn.is_empty() || self.alpn.len() > 16 {
            return Err(TlsCamouflageError::InvalidHelloField);
        }
        let alpn_len =
            u8::try_from(self.alpn.len()).map_err(|_| TlsCamouflageError::InvalidHelloField)?;

        let cipher_len = u8::try_from(self.cipher_suites.len())
            .map_err(|_| TlsCamouflageError::InvalidHelloField)?;
        let ext_len = u8::try_from(self.extensions.len())
            .map_err(|_| TlsCamouflageError::InvalidHelloField)?;

        let mut encoded = Vec::with_capacity(128);
        encoded.push(alpn_len);
        encoded.extend_from_slice(self.alpn.as_bytes());
        encoded.extend_from_slice(&self.session_id);
        encoded.push(cipher_len);
        for suite in &self.cipher_suites {
            encoded.extend_from_slice(&suite.to_be_bytes());
        }
        encoded.push(ext_len);
        for extension in &self.extensions {
            encoded.extend_from_slice(&extension.to_be_bytes());
        }

        Ok(encoded)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::profiles::CHROME_131;
    use crate::config::profiles::builtin_profile;
    use crate::stealth::client_hello::ClientHelloSpec;

    #[test]
    fn client_hello_uses_profile_alpn() {
        let profile = builtin_profile(CHROME_131).expect("profile");
        let hello = ClientHelloSpec::from_profile(&profile);
        assert_eq!(profile.alpn, hello.alpn);
    }

    #[test]
    fn client_hello_encoding_contains_cipher_list() {
        let profile = builtin_profile(CHROME_131).expect("profile");
        let hello = ClientHelloSpec::from_profile(&profile);
        let encoded = hello.encode().expect("encoded");

        assert!(encoded.len() > 40);
    }
}
