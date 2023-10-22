use bats_dsp::{moog_filter::MoogFilter, SampleRate};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

const BUFFER_SIZE: usize = 128;
const SAMPLE_RATE: f32 = 44100.0;

fn moog_filter_benchmark(c: &mut Criterion) {
    c.bench_function("moog_filter_init", |b| {
        b.iter(|| {
            let _ = MoogFilter::new(SampleRate::new(SAMPLE_RATE));
        })
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

criterion_group!(benches, moog_filter_benchmark);
criterion_main!(benches);
