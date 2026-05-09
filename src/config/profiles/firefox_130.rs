use crate::config::profiles::{FIREFOX_130, StealthProfile};

pub fn profile() -> StealthProfile {
    StealthProfile {
        name: String::from(FIREFOX_130),
        alpn: String::from("h2"),
        min_packet_size: 820,
        max_packet_size: 1280,
        min_jitter_ms: 3,
        max_jitter_ms: 16,
        cipher_suites: vec![
            0x1301, 0x1303, 0x1302, 0xC02B, 0xC02F, 0xC02C, 0xC030, 0xCCA9, 0xCCA8, 0xC013, 0xC014,
        ],
        extensions: vec![
            0x0000, 0x0017, 0xFF01, 0x000A, 0x000B, 0x0023, 0x0010, 0x0005, 0x000D, 0x0033, 0x002B,
            0x002D, 0x001C,
        ],
        traffic_profile: Some(String::from("firefox_h3")),
    }
}
