use crate::stealth::entropy::EntropySource;

pub struct DecoyStreamGenerator {
    entropy: EntropySource,
    enabled: bool,
    next_decoy_at_packet: u32,
    packet_counter: u32,
}

impl DecoyStreamGenerator {
    pub fn new(enabled: bool) -> Self {
        let mut entropy = EntropySource::new();
        let next = entropy.random_in_range(10, 50) as u32;
        Self {
            entropy,
            enabled,
            next_decoy_at_packet: next,
            packet_counter: 0,
        }
    }

    pub fn on_packet_sent(&mut self) {
        self.packet_counter += 1;
    }

    pub fn should_inject_decoy(&mut self) -> Option<DecoyPayload> {
        if !self.enabled {
            return None;
        }

        if self.packet_counter < self.next_decoy_at_packet {
            return None;
        }

        self.packet_counter = 0;
        self.next_decoy_at_packet = self.entropy.random_in_range(10, 50) as u32;

        let decoy_type = self.entropy.random_in_range(0, 2);
        Some(match decoy_type {
            0 => self.generate_fake_get(),
            1 => self.generate_fake_post(),
            _ => self.generate_fake_response(),
        })
    }

    fn generate_fake_get(&mut self) -> DecoyPayload {
        let paths = [
            "/api/v1/status",
            "/assets/main.js",
            "/favicon.ico",
            "/api/metrics",
            "/.well-known/security.txt",
            "/robots.txt",
            "/sitemap.xml",
        ];
        let idx = self.entropy.random_in_range(0, (paths.len() - 1) as u16) as usize;
        let path = paths[idx];

        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: cdn.example.com\r\nAccept: */*\r\nConnection: keep-alive\r\n\r\n"
        );
        DecoyPayload {
            data: request.into_bytes(),
            kind: DecoyKind::HttpGet,
        }
    }

    fn generate_fake_post(&mut self) -> DecoyPayload {
        let body_size = self.entropy.random_in_range(32, 256) as usize;
        let mut body = vec![0u8; body_size];
        self.entropy.fill_padding(&mut body);
        let body_hex: String = body.iter().map(|b| format!("{b:02x}")).collect();

        let request = format!(
            "POST /api/v1/telemetry HTTP/1.1\r\nHost: cdn.example.com\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
            body_hex.len(),
            body_hex,
        );
        DecoyPayload {
            data: request.into_bytes(),
            kind: DecoyKind::HttpPost,
        }
    }

    fn generate_fake_response(&mut self) -> DecoyPayload {
        let body_size = self.entropy.random_in_range(64, 512) as usize;
        let mut body = vec![0u8; body_size];
        self.entropy.fill_padding(&mut body);
        let body_hex: String = body.iter().map(|b| format!("{b:02x}")).collect();

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body_hex.len(),
            body_hex,
        );
        DecoyPayload {
            data: response.into_bytes(),
            kind: DecoyKind::HttpResponse,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoyKind {
    HttpGet,
    HttpPost,
    HttpResponse,
}

#[derive(Debug, Clone)]
pub struct DecoyPayload {
    pub data: Vec<u8>,
    pub kind: DecoyKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoy_generator_produces_payloads_when_enabled() {
        let mut decoy = DecoyStreamGenerator::new(true);
        let mut found_decoy = false;

        for _ in 0..200 {
            decoy.on_packet_sent();
            if decoy.should_inject_decoy().is_some() {
                found_decoy = true;
                break;
            }
        }

        assert!(found_decoy);
    }

    #[test]
    fn decoy_generator_silent_when_disabled() {
        let mut decoy = DecoyStreamGenerator::new(false);

        for _ in 0..200 {
            decoy.on_packet_sent();
            assert!(decoy.should_inject_decoy().is_none());
        }
    }

    #[test]
    fn decoy_payload_contains_http_content() {
        let mut decoy = DecoyStreamGenerator::new(true);
        decoy.packet_counter = 100;
        decoy.next_decoy_at_packet = 1;

        let payload = decoy.should_inject_decoy().expect("decoy");
        let text = String::from_utf8_lossy(&payload.data);
        assert!(text.contains("HTTP") || text.contains("GET") || text.contains("POST"));
    }
}
