#![no_main]

use apate::config::parser::parse_config;
use libfuzzer_sys::fuzz_target;

// Repro: cargo +nightly fuzz run config_parser -- fuzz/corpus/config_parser
// Minimize: cargo +nightly fuzz tmin config_parser fuzz/artifacts/config_parser/<crash> -o /tmp/config_min
fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = parse_config(input);
    }
});
