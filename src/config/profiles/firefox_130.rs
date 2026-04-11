use crate::config::profiles::{FIREFOX_130, StealthProfile};

pub fn profile() -> StealthProfile {
    StealthProfile {
        name: String::from(FIREFOX_130),
        alpn: String::from("h2"),
        min_packet_size: 820,
        max_packet_size: 1280,
        min_jitter_ms: 3,
        max_jitter_ms: 16,
    }
}
