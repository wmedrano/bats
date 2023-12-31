use std::time::Duration;

use bats_dsp::{buffers::Buffers, sample_rate::SampleRate};
use bats_lib::{
    builder::BatsBuilder,
    plugin::{toof::Toof, BatsInstrument, BatsInstrumentExt},
};
use bmidi::{Channel, MidiMessage, Note, U7};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

const BUFFER_SIZE: usize = 128;
const SAMPLE_RATE: f32 = 44100.0;
const PRESS_C4: MidiMessage = MidiMessage::NoteOn(Channel::Ch1, Note::C4, U7::MAX);
const RELEASE_C4: MidiMessage = MidiMessage::NoteOff(Channel::Ch1, Note::C4, U7::MIN);
const PRESS_A4: MidiMessage = MidiMessage::NoteOn(Channel::Ch1, Note::A4, U7::MAX);
const RELEASE_A4: MidiMessage = MidiMessage::NoteOff(Channel::Ch1, Note::A4, U7::MIN);

fn bats_init_benchmark(c: &mut Criterion) {
    // Does not need high precision. Just enough to deterimine if initialization is a snappy user
    // experience.
    c.benchmark_group("lib-init")
        .warm_up_time(Duration::from_millis(10))
        .measurement_time(Duration::from_secs(1))
        .confidence_level(0.99)
        .bench_function("bats", |b| {
            b.iter(|| {
                BatsBuilder {
                    sample_rate: SampleRate::new(SAMPLE_RATE),
                    buffer_size: BUFFER_SIZE,
                    bpm: 120.0,
                    tracks: Default::default(),
                }
                .build()
            })
        });
}

fn bats_benchmark(c: &mut Criterion) {
    c.benchmark_group("lib-bats")
        .measurement_time(Duration::from_secs(10))
        .confidence_level(0.99)
        .bench_function("empty", |b| {
            let mut bats = BatsBuilder {
                sample_rate: SampleRate::new(SAMPLE_RATE),
                buffer_size: BUFFER_SIZE,
                bpm: 120.0,
                tracks: Default::default(),
            }
            .build();
            let mut buffers = black_box(Buffers::new(BUFFER_SIZE));
            let midi = black_box(&[]);
            b.iter(move || {
                bats.process(midi, &mut buffers.left, &mut buffers.right);
            })
        })
        .bench_function("bats_with_8_toofs", |b| {
            let mut bats = black_box(
                BatsBuilder {
                    sample_rate: SampleRate::new(SAMPLE_RATE),
                    buffer_size: BUFFER_SIZE,
                    bpm: 120.0,
                    tracks: Default::default(),
                }
                .build(),
            );
            for track in bats.tracks.iter_mut().take(8) {
                track.plugin = Toof::new(bats.sample_rate).into();
            }
            let mut buffers = black_box(Buffers::new(BUFFER_SIZE));
            let midi = black_box([
                (0, PRESS_C4.clone()),
                (BUFFER_SIZE as u32 / 2, RELEASE_C4.clone()),
            ]);
            let midi_ref = black_box(&midi);
            b.iter(move || {
                bats.process(midi_ref, &mut buffers.left, &mut buffers.right);
            })
        });
}

fn transport_benchmark(c: &mut Criterion) {
    c.benchmark_group("lib-transport")
        .measurement_time(Duration::from_secs(1))
        .confidence_level(0.99)
        .bench_function("tick", |b| {
            let mut transport = black_box(bats_lib::transport::Transport::new(
                SampleRate::new(SAMPLE_RATE),
                BUFFER_SIZE,
                120.0,
            ));
            let mut buffers = Buffers::new(BUFFER_SIZE);
            b.iter(move || {
                transport.process(&mut buffers.left, &mut buffers.right);
            })
        });
}

fn toof_benchmark(c: &mut Criterion) {
    c.benchmark_group("lib-toof")
        .measurement_time(Duration::from_secs(5))
        .confidence_level(0.99)
        .bench_function("process", |b| {
            let mut toof = black_box(Toof::new(SampleRate::new(SAMPLE_RATE)));
            let mut buffers = black_box(Buffers::new(BUFFER_SIZE));
            let midi = black_box([
                (0, PRESS_C4.clone()),
                (2 * BUFFER_SIZE as u32 / 4, PRESS_A4.clone()),
                (3 * BUFFER_SIZE as u32 / 4, RELEASE_C4.clone()),
                ((4 * BUFFER_SIZE as u32 - 1) / 4, RELEASE_A4.clone()),
            ]);
            let midi_ref = black_box(&midi);
            b.iter(move || {
                toof.process_batch(midi_ref, &mut buffers);
            })
        })
        .bench_function("process-no-filter", |b| {
            let mut toof = black_box(Toof::new(SampleRate::new(SAMPLE_RATE)));
            toof.set_param_by_name("bypass filter", 1.0).unwrap();
            let mut buffers = black_box(Buffers::new(BUFFER_SIZE));
            let midi = black_box([
                (0, PRESS_C4.clone()),
                (2 * BUFFER_SIZE as u32 / 4, PRESS_A4.clone()),
                (3 * BUFFER_SIZE as u32 / 4, RELEASE_C4.clone()),
                ((4 * BUFFER_SIZE as u32 - 1) / 4, RELEASE_A4.clone()),
            ]);
            let midi_ref = black_box(&midi);
            b.iter(move || {
                toof.process_batch(midi_ref, &mut buffers);
            })
        });
}

criterion_group!(
    benches,
    bats_init_benchmark,
    bats_benchmark,
    transport_benchmark,
    toof_benchmark
);
criterion_main!(benches);
