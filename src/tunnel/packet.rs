use crate::tunnel::TunnelError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpVersion {
    V4,
    V6,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelPacket {
    bytes: Vec<u8>,
    ip_version: IpVersion,
}

impl TunnelPacket {
    pub fn parse(bytes: &[u8]) -> Result<Self, TunnelError> {
        let first = *bytes.first().ok_or(TunnelError::InvalidPacket)?;
        let version = first >> 4;

        match version {
            4 => validate_ipv4(bytes)?,
            6 => validate_ipv6(bytes)?,
            _ => return Err(TunnelError::InvalidPacket),
        }

        let ip_version = if version == 4 {
            IpVersion::V4
        } else {
            IpVersion::V6
        };

        Ok(Self {
            bytes: bytes.to_vec(),
            ip_version,
        })
    }

    pub fn ip_version(&self) -> IpVersion {
        self.ip_version
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

fn validate_ipv4(bytes: &[u8]) -> Result<(), TunnelError> {
    if bytes.len() < 20 {
        return Err(TunnelError::InvalidPacket);
    }

    let ihl_words = bytes[0] & 0x0F;
    if ihl_words < 5 {
        return Err(TunnelError::InvalidPacket);
    }

    let header_len = usize::from(ihl_words) * 4;
    let total_len = usize::from(u16::from_be_bytes([bytes[2], bytes[3]]));
    if total_len < header_len || total_len > bytes.len() {
        return Err(TunnelError::InvalidPacket);
    }

    Ok(())
}

fn validate_ipv6(bytes: &[u8]) -> Result<(), TunnelError> {
    if bytes.len() < 40 {
        return Err(TunnelError::InvalidPacket);
    }

    let payload_len = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    let expected = 40_usize.saturating_add(payload_len);
    if expected > bytes.len() {
        return Err(TunnelError::InvalidPacket);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::tunnel::packet::{IpVersion, TunnelPacket};

    #[test]
    fn parse_valid_ipv4_packet() {
        let packet = [
            0x45, 0x00, 0x00, 0x14, 0, 0, 0, 0, 64, 6, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
        ];

        let parsed = TunnelPacket::parse(&packet).expect("ipv4 parse");
        assert_eq!(IpVersion::V4, parsed.ip_version());
    }

    #[test]
    fn parse_valid_ipv6_packet() {
        let mut packet = vec![0_u8; 40];
        packet[0] = 0x60;
        packet[4] = 0;
        packet[5] = 0;

        let parsed = TunnelPacket::parse(&packet).expect("ipv6 parse");
        assert_eq!(IpVersion::V6, parsed.ip_version());
    }

    #[test]
    fn reject_truncated_ipv6_payload() {
        let mut packet = vec![0_u8; 40];
        packet[0] = 0x60;
        packet[4] = 0;
        packet[5] = 10;

        assert!(TunnelPacket::parse(&packet).is_err());
    }
}
