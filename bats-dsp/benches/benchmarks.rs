use std::time::Duration;

use bats_dsp::{
    envelope::{Envelope, EnvelopeParams},
    moog_filter::MoogFilter,
    sample_rate::SampleRate,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

const BUFFER_SIZE: usize = 128;
const SAMPLE_RATE: f32 = 44100.0;

fn init_benchmark(c: &mut Criterion) {
    c.benchmark_group("init")
        .warm_up_time(Duration::from_millis(10))
        .measurement_time(Duration::from_millis(100))
        .confidence_level(0.99)
        .bench_function("moog", |b| {
            b.iter(|| MoogFilter::new(SampleRate::new(black_box(SAMPLE_RATE))))
        })
        .bench_function("envelope", |b| {
            b.iter(|| {
                EnvelopeParams::new(
                    black_box(SampleRate::new(SAMPLE_RATE)),
                    black_box(1.0),
                    black_box(1.0),
                    black_box(1.0),
                    black_box(1.0),
                )
            })
        });
}

fn moog_filter_benchmark(c: &mut Criterion) {
    c.benchmark_group("moog")
        .measurement_time(Duration::from_secs(1))
        .confidence_level(0.99)
        .bench_function("process", |b| {
            let mut f = MoogFilter::new(SampleRate::new(SAMPLE_RATE));
            let mut result = black_box(vec![0f32; BUFFER_SIZE]);
            b.iter(move || {
                for out in result.iter_mut() {
                    *out = f.process(*out);
                }
            });
        });
}

fn envelope_benchmark(c: &mut Criterion) {
    c.benchmark_group("envelope")
        .measurement_time(Duration::from_secs(1))
        .confidence_level(0.99)
        .bench_function("process", |b| {
            let mut result = black_box(vec![0f32; BUFFER_SIZE]);
            let params = black_box(EnvelopeParams::new(
                SampleRate::new(SAMPLE_RATE),
                1.0,
                1.0,
                1.0,
                1.0,
            ));
            b.iter(move || {
                let mut envelope = Envelope::new();
                for out in result.iter_mut() {
                    *out = envelope.next_sample(&params);
                }
            });
        });
}

criterion_group!(
    benches,
    init_benchmark,
    moog_filter_benchmark,
    envelope_benchmark
);
criterion_main!(benches);
