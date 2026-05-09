use apate::crypto::kx::derive_public_key;
use apate::crypto::sign::{derive_verifying_key, sign_message};
use apate::noise::handshake::{HandshakeMachine, HandshakeMessage};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

fn handshake_benchmarks(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("handshake");

    let server_signing_key = [42u8; 32];
    let server_verifying_key = derive_verifying_key(server_signing_key);
    let client_secret = [3u8; 32];
    let server_secret = [9u8; 32];
    let client_eph_pub = derive_public_key(client_secret);
    let server_eph_pub = derive_public_key(server_secret);

    group.throughput(Throughput::Elements(3));
    group.bench_with_input(
        BenchmarkId::new("full_handshake_roundtrip", 3),
        &(client_eph_pub, server_eph_pub, server_signing_key, server_verifying_key),
        |bencher, &(client_pub, server_pub, signing_key, verifying_key)| {
            bencher.iter(|| {
                let mut machine = HandshakeMachine::new(verifying_key);

                machine
                    .process(HandshakeMessage::ClientHello {
                        ephemeral_public: client_pub,
                    })
                    .expect("client hello");

                machine
                    .process(HandshakeMessage::ServerHello {
                        ephemeral_public: server_pub,
                    })
                    .expect("server hello");

                let sig = sign_message(signing_key, &machine.symmetric_state.handshake_hash);

                machine
                    .process(HandshakeMessage::AuthProof { signature: sig })
                    .expect("auth proof");
            });
        },
    );

    group.finish();
}

criterion_group!(benches, handshake_benchmarks);
criterion_main!(benches);
