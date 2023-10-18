use criterion::{criterion_group, criterion_main, Criterion};

const BUFFER_SIZES: [usize; 3] = [64, 128, 256];

fn criterion_benchmark(c: &mut Criterion) {
    for buffer_size in BUFFER_SIZES {
        c.bench_function(&format!("empty_{buffer_size}"), |b| {
            let mut bats = bats_lib::Bats::default();
            let (mut left, mut right) = make_buffers(buffer_size);
            b.iter(move || {
                bats.process(std::iter::empty(), &mut left, &mut right);
            })
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

fn make_buffers(buffer_size: usize) -> (Vec<f32>, Vec<f32>) {
    (vec![0f32; buffer_size], vec![0f32; buffer_size])
}
