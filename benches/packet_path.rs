use apate::transport::frame::{Frame, FrameType, decode_frame, encode_frame};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

fn packet_path_benchmarks(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("packet_path");

    for payload_len in [256usize, 1024usize, 4096usize] {
        let frame = Frame {
            frame_type: FrameType::Data,
            sequence: 7,
            payload: vec![0xAB; payload_len],
        };
        let encoded = encode_frame(&frame, 0).expect("encode");

        group.throughput(Throughput::Bytes(payload_len as u64));
        group.bench_with_input(
            BenchmarkId::new("encode_frame", payload_len),
            &frame,
            |bencher, input| {
                bencher.iter(|| encode_frame(input, 0).expect("encode"));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("decode_frame", payload_len),
            &encoded,
            |bencher, input| {
                bencher.iter(|| decode_frame(input).expect("decode"));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, packet_path_benchmarks);
criterion_main!(benches);
