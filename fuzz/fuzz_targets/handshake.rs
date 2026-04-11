#![no_main]

use apate::noise::handshake::{HandshakeMachine, HandshakeMessage};
use libfuzzer_sys::fuzz_target;

fn read_fixed<const N: usize>(data: &[u8], cursor: &mut usize) -> [u8; N] {
    let mut out = [0_u8; N];
    let remaining = data.len().saturating_sub(*cursor);
    let take = remaining.min(N);
    if take > 0 {
        out[..take].copy_from_slice(&data[*cursor..(*cursor + take)]);
        *cursor += take;
    }
    out
}

// Repro: cargo +nightly fuzz run handshake -- fuzz/corpus/handshake
// Minimize: cargo +nightly fuzz tmin handshake fuzz/artifacts/handshake/<crash> -o /tmp/handshake_min
fuzz_target!(|data: &[u8]| {
    let mut machine = HandshakeMachine::default();
    let mut cursor = 0usize;
    let max_steps = 16usize;

    for _ in 0..max_steps {
        if cursor >= data.len() {
            break;
        }

        let tag = data[cursor] % 3;
        cursor += 1;
        let message = match tag {
            0 => HandshakeMessage::ClientHello {
                ephemeral_public: read_fixed::<32>(data, &mut cursor),
            },
            1 => HandshakeMessage::ServerHello {
                ephemeral_public: read_fixed::<32>(data, &mut cursor),
            },
            _ => HandshakeMessage::AuthProof {
                signature: read_fixed::<64>(data, &mut cursor),
            },
        };

        let _ = machine.process(message);
    }
});
