use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DohForwarder {
    endpoint: String,
}

impl DohForwarder {
    pub fn new(endpoint: String) -> Result<Self, DohError> {
        if !endpoint.starts_with("https://") || endpoint.trim().len() <= "https://".len() {
            return Err(DohError::InvalidEndpoint);
        }

        Ok(Self { endpoint })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn build_query(
        &self,
        domain: &str,
        record_type: u16,
        request_id: u16,
    ) -> Result<Vec<u8>, DohError> {
        validate_domain(domain)?;
        if record_type == 0 {
            return Err(DohError::InvalidRecordType);
        }

        let mut query = Vec::with_capacity(512);
        query.extend_from_slice(&request_id.to_be_bytes());
        query.extend_from_slice(&0x0100_u16.to_be_bytes());
        query.extend_from_slice(&1_u16.to_be_bytes());
        query.extend_from_slice(&0_u16.to_be_bytes());
        query.extend_from_slice(&0_u16.to_be_bytes());
        query.extend_from_slice(&0_u16.to_be_bytes());

        for label in domain.split('.') {
            let label_len = u8::try_from(label.len()).map_err(|_| DohError::InvalidDomain)?;
            query.push(label_len);
            query.extend_from_slice(label.as_bytes());
        }
        query.push(0);
        query.extend_from_slice(&record_type.to_be_bytes());
        query.extend_from_slice(&1_u16.to_be_bytes());

        Ok(query)
    }

    pub fn parse_response(&self, response: &[u8]) -> Result<DohResponse, DohError> {
        if response.len() < 12 {
            return Err(DohError::MalformedResponse);
        }

        let request_id = u16::from_be_bytes([response[0], response[1]]);
        let flags = u16::from_be_bytes([response[2], response[3]]);
        let answer_count = u16::from_be_bytes([response[6], response[7]]);
        let rcode = (flags & 0x000F) as u8;

        Ok(DohResponse {
            request_id,
            answer_count,
            rcode,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DohResponse {
    pub request_id: u16,
    pub answer_count: u16,
    pub rcode: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DohError {
    #[error("invalid doh endpoint")]
    InvalidEndpoint,
    #[error("invalid dns domain")]
    InvalidDomain,
    #[error("invalid dns record type")]
    InvalidRecordType,
    #[error("malformed dns response")]
    MalformedResponse,
}

fn validate_domain(domain: &str) -> Result<(), DohError> {
    if domain.is_empty() || domain.len() > 253 {
        return Err(DohError::InvalidDomain);
    }

    for label in domain.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(DohError::InvalidDomain);
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(DohError::InvalidDomain);
        }
        if !label
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        {
            return Err(DohError::InvalidDomain);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::routing::doh::{DohError, DohForwarder};

    #[test]
    fn forwarder_rejects_non_https_endpoint() {
        let created = DohForwarder::new(String::from("http://resolver.example/dns-query"));
        assert_eq!(Err(DohError::InvalidEndpoint), created);
    }

    #[test]
    fn forwarder_builds_dns_query_for_dual_stack() {
        let forwarder = DohForwarder::new(String::from("https://resolver.example/dns-query"))
            .expect("forwarder");
        let a_query = forwarder.build_query("example.com", 1, 0x1234).expect("a");
        let aaaa_query = forwarder
            .build_query("example.com", 28, 0x1235)
            .expect("aaaa");

        assert!(a_query.len() > 12);
        assert!(aaaa_query.len() > 12);
        assert_eq!(
            1_u16.to_be_bytes(),
            [a_query[a_query.len() - 4], a_query[a_query.len() - 3]]
        );
        assert_eq!(
            28_u16.to_be_bytes(),
            [
                aaaa_query[aaaa_query.len() - 4],
                aaaa_query[aaaa_query.len() - 3]
            ]
        );
    }

    #[test]
    fn forwarder_parses_minimal_dns_response_header() {
        let forwarder = DohForwarder::new(String::from("https://resolver.example/dns-query"))
            .expect("forwarder");
        let response = [
            0x12, 0x34, 0x81, 0x80, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
        ];

        let parsed = forwarder.parse_response(&response).expect("parsed");
        assert_eq!(0x1234, parsed.request_id);
        assert_eq!(2, parsed.answer_count);
        assert_eq!(0, parsed.rcode);
    }
}
