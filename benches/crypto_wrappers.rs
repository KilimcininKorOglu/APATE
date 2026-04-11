use apate::crypto::aead::{decrypt_chacha20poly1305, encrypt_chacha20poly1305};
use apate::crypto::kx::{derive_public_key, derive_shared_secret};
use apate::crypto::sign::{derive_verifying_key, sign_message, verify_message};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

fn crypto_wrapper_benchmarks(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("crypto_wrappers");

    let key = [7_u8; 32];
    let nonce = [1_u8; 12];
    let aad = b"apate-bench";
    let payload = vec![0xCD; 1024];
    let ciphertext = encrypt_chacha20poly1305(&key, &nonce, &payload, aad).expect("encrypt");

    group.throughput(Throughput::Bytes(payload.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("encrypt_chacha20poly1305", payload.len()),
        &payload,
        |bencher, input| {
            bencher.iter(|| encrypt_chacha20poly1305(&key, &nonce, input, aad).expect("encrypt"));
        },
    );
    group.bench_with_input(
        BenchmarkId::new("decrypt_chacha20poly1305", payload.len()),
        &ciphertext,
        |bencher, input| {
            bencher.iter(|| decrypt_chacha20poly1305(&key, &nonce, input, aad).expect("decrypt"));
        },
    );

    let signing_key = [42_u8; 32];
    let verifying_key = derive_verifying_key(signing_key);
    let message = b"apate benchmark signing payload";
    let signature = sign_message(signing_key, message);

    group.bench_function("sign_message_ed25519", |bencher| {
        bencher.iter(|| sign_message(signing_key, message))
    });
    group.bench_function("verify_message_ed25519", |bencher| {
        bencher.iter(|| verify_message(verifying_key, message, signature).expect("verify"))
    });

    let alice_secret = [3_u8; 32];
    let bob_secret = [9_u8; 32];
    let bob_public = derive_public_key(bob_secret);
    let _ = derive_shared_secret(alice_secret, bob_public).expect("shared");

    group.bench_function("derive_shared_secret_x25519", |bencher| {
        bencher
            .iter(|| derive_shared_secret(alice_secret, bob_public).expect("shared secret derive"))
    });

    group.finish();
}

criterion_group!(benches, crypto_wrapper_benchmarks);
criterion_main!(benches);
