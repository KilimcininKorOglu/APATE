use crate::stealth::tls_camouflage::TlsCamouflageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerHelloSpec {
    pub selected_cipher: u16,
    pub selected_extension: u16,
    pub alpn: String,
}

impl ServerHelloSpec {
    pub fn new(alpn: String, selected_cipher: u16, selected_extension: u16) -> Self {
        Self {
            selected_cipher,
            selected_extension,
            alpn,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, TlsCamouflageError> {
        if self.alpn.is_empty() || self.alpn.len() > 16 {
            return Err(TlsCamouflageError::InvalidHelloField);
        }
        let alpn_len =
            u8::try_from(self.alpn.len()).map_err(|_| TlsCamouflageError::InvalidHelloField)?;

        let mut encoded = Vec::with_capacity(16);
        encoded.extend_from_slice(&self.selected_cipher.to_be_bytes());
        encoded.extend_from_slice(&self.selected_extension.to_be_bytes());
        encoded.push(alpn_len);
        encoded.extend_from_slice(self.alpn.as_bytes());

        Ok(encoded)
    }
}

#[cfg(test)]
mod tests {
    use crate::stealth::server_hello::ServerHelloSpec;

    #[test]
    fn server_hello_encodes_selected_parameters() {
        let hello = ServerHelloSpec::new(String::from("h2"), 0x1301, 0x0010);
        let encoded = hello.encode().expect("encoded");

        assert!(encoded.len() >= 7);
        assert_eq!([0x13, 0x01], [encoded[0], encoded[1]]);
    }
}
