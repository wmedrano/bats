use bats_dsp::{
    envelope::{Envelope, EnvelopeParams},
    moog_filter::MoogFilter,
    sample_rate::SampleRate,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

const BUFFER_SIZE: usize = 128;
const SAMPLE_RATE: f32 = 44100.0;

fn moog_filter_benchmark(c: &mut Criterion) {
    c.bench_function("moog_filter_init", |b| {
        b.iter(|| MoogFilter::new(SampleRate::new(black_box(SAMPLE_RATE))))
    });
    c.bench_function("moog_filter_process", |b| {
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
    c.bench_function("envelope_init", |b| {
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
    c.bench_function("envelope_process", |b| {
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

criterion_group!(benches, moog_filter_benchmark, envelope_benchmark);
criterion_main!(benches);
