use criterion::{criterion_group, criterion_main, Criterion};

const BUFFER_SIZES: [usize; 3] = [64, 128, 256];
const DEFAULT_SAMPLE_RATE: f32 = 44100.0;

fn bats_benchmark(c: &mut Criterion) {
    for buffer_size in BUFFER_SIZES {
        c.bench_function(&format!("bats_empty_{buffer_size}"), |b| {
            let mut bats = bats_lib::Bats::new(DEFAULT_SAMPLE_RATE, buffer_size);
            let (mut left, mut right) = make_buffers(buffer_size);
            b.iter(move || {
                bats.process(std::iter::empty(), &mut left, &mut right);
            })
        });
    }
}

fn metronome_benchmark(c: &mut Criterion) {
    for buffer_size in BUFFER_SIZES {
        c.bench_function(&format!("metronome_tick_{buffer_size}"), |b| {
            let mut metronome = bats_lib::metronome::Metronome::new(DEFAULT_SAMPLE_RATE, 120.0);
            b.iter(move || {
                for _ in 0..buffer_size {
                    metronome.next_position();
                }
            })
        });
    }
}

criterion_group!(benches, bats_benchmark, metronome_benchmark);
criterion_main!(benches);

fn make_buffers(buffer_size: usize) -> (Vec<f32>, Vec<f32>) {
    (vec![0f32; buffer_size], vec![0f32; buffer_size])
}
