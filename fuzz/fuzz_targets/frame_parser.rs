#![no_main]

use apate::transport::frame::{decode_frame, encode_frame};
use libfuzzer_sys::fuzz_target;

// Repro: cargo +nightly fuzz run frame_parser -- fuzz/corpus/frame_parser
// Minimize: cargo +nightly fuzz tmin frame_parser fuzz/artifacts/frame_parser/<crash> -o /tmp/frame_min
fuzz_target!(|data: &[u8]| {
    if let Ok(decoded) = decode_frame(data) {
        if let Ok(reencoded) = encode_frame(&decoded.frame, decoded.context.flags) {
            let _ = decode_frame(&reencoded);
        }
    }
});
