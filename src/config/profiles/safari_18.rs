use crate::config::profiles::{SAFARI_18, StealthProfile};

pub fn profile() -> StealthProfile {
    StealthProfile {
        name: String::from(SAFARI_18),
        alpn: String::from("h2"),
        min_packet_size: 760,
        max_packet_size: 1240,
        min_jitter_ms: 5,
        max_jitter_ms: 22,
    }
}
