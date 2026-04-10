#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Init,
    Handshaking,
    Established,
    Rekeying,
    Migrating,
    Closing,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    Auto,
    Udp,
    Tcp,
}

impl TransportMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Udp => "udp",
            Self::Tcp => "tcp",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "udp" => Some(Self::Udp),
            "tcp" => Some(Self::Tcp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    StaticKey,
    Token,
    Certificate,
}

impl AuthMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StaticKey => "static_key",
            Self::Token => "token",
            Self::Certificate => "certificate",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "static_key" => Some(Self::StaticKey),
            "token" => Some(Self::Token),
            "certificate" => Some(Self::Certificate),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CamouflageMode {
    TlsCamouflage,
    QuicMask,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionSession {
    pub connection_id: [u8; 16],
    pub state: ConnectionState,
    pub transport_mode: TransportMode,
    pub auth_method: AuthMethod,
    pub established_at_unix: u64,
    pub peer_endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoContext {
    pub key_epoch: u64,
    pub tx_nonce_counter: u64,
    pub rekey_interval_secs: u32,
    pub rekey_interval_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StealthProfile {
    pub name: String,
    pub mode: CamouflageMode,
    pub packet_min: u16,
    pub packet_max: u16,
    pub jitter_ms_max: u16,
}

#[cfg(test)]
mod tests {
    use super::{AuthMethod, TransportMode};

    #[test]
    fn transport_mode_roundtrip_parse() {
        let mode = TransportMode::Udp;
        let as_str = mode.as_str();

        assert_eq!(Some(mode), TransportMode::parse(as_str));
    }

    #[test]
    fn auth_method_roundtrip_parse() {
        let method = AuthMethod::Certificate;
        let as_str = method.as_str();

        assert_eq!(Some(method), AuthMethod::parse(as_str));
    }
}
