use crate::config::profiles::{CHROME_131, StealthProfile};

pub fn profile() -> StealthProfile {
    StealthProfile {
        name: String::from(CHROME_131),
        alpn: String::from("h2"),
        min_packet_size: 900,
        max_packet_size: 1350,
        min_jitter_ms: 4,
        max_jitter_ms: 18,
    }
}
