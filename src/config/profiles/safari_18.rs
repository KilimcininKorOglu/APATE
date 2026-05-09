use crate::config::profiles::{SAFARI_18, StealthProfile};

pub fn profile() -> StealthProfile {
    StealthProfile {
        name: String::from(SAFARI_18),
        alpn: String::from("h2"),
        min_packet_size: 760,
        max_packet_size: 1240,
        min_jitter_ms: 5,
        max_jitter_ms: 22,
        cipher_suites: vec![
            0x1301, 0x1302, 0x1303, 0xC02C, 0xC02B, 0xC030, 0xC02F, 0xCCA9, 0xCCA8,
        ],
        extensions: vec![
            0x0000, 0xFF01, 0x000A, 0x000B, 0x0023, 0x0010, 0x0005, 0x0012, 0x0033, 0x002B,
            0x002D,
        ],
    }
}
