use apate::noise::handshake::{HandshakeMachine, HandshakeMessage};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

fn handshake_benchmarks(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("handshake");

    let handshake_messages = [
        HandshakeMessage::ClientHello {
            ephemeral_public: [1_u8; 32],
        },
        HandshakeMessage::ServerHello {
            ephemeral_public: [2_u8; 32],
        },
        HandshakeMessage::AuthProof {
            signature: [9_u8; 64],
        },
    ];

    group.throughput(Throughput::Elements(handshake_messages.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("full_handshake_roundtrip", handshake_messages.len()),
        &handshake_messages,
        |bencher, input| {
            bencher.iter(|| {
                let mut machine = HandshakeMachine::default();
                for message in input.iter().cloned() {
                    let _ = machine.process(message).expect("process handshake");
                }
            });
        },
    );

    group.finish();
}

criterion_group!(benches, handshake_benchmarks);
criterion_main!(benches);
