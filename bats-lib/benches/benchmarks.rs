use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wmidi::{Channel, MidiMessage, Note, U7};

const BUFFER_SIZES: [usize; 3] = [64, 128, 256];
const DEFAULT_SAMPLE_RATE: f32 = 44100.0;
const PRESS_C4: MidiMessage<'static> = MidiMessage::NoteOn(Channel::Ch1, Note::C4, U7::MAX);
const RELEASE_C4: MidiMessage<'static> = MidiMessage::NoteOff(Channel::Ch1, Note::C4, U7::MIN);
const PRESS_A4: MidiMessage<'static> = MidiMessage::NoteOn(Channel::Ch1, Note::A4, U7::MAX);
const RELEASE_A4: MidiMessage<'static> = MidiMessage::NoteOff(Channel::Ch1, Note::A4, U7::MIN);

fn bats_benchmark(c: &mut Criterion) {
    {
        let buffer_size = *BUFFER_SIZES.last().unwrap();
        c.bench_function(&format!("bats_init_{buffer_size}"), |b| {
            b.iter(|| {
                let _ = bats_lib::Bats::new(DEFAULT_SAMPLE_RATE, buffer_size);
            })
        });
    }
    for buffer_size in BUFFER_SIZES {
        c.bench_function(&format!("bats_empty_{buffer_size}"), |b| {
            let mut bats = bats_lib::Bats::new(DEFAULT_SAMPLE_RATE, buffer_size);
            let (mut left, mut right) = make_buffers(buffer_size);
            let midi = black_box(&[]);
            b.iter(move || {
                bats.process(midi, &mut left, &mut right);
            })
        });
    }
    for buffer_size in BUFFER_SIZES {
        c.bench_function(&format!("bats_with_plugins_{buffer_size}"), |b| {
            let mut bats = black_box(bats_lib::Bats::new(DEFAULT_SAMPLE_RATE, buffer_size));
            let (mut left, mut right) = make_buffers(buffer_size);
            let midi = black_box([
                (0, PRESS_C4.clone()),
                (buffer_size as u32 / 2, RELEASE_C4.clone()),
            ]);
            let midi_ref = black_box(&midi);
            b.iter(move || {
                bats.process(midi_ref, &mut left, &mut right);
            })
        });
    }
}

fn metronome_benchmark(c: &mut Criterion) {
    for buffer_size in BUFFER_SIZES {
        c.bench_function(&format!("metronome_tick_{buffer_size}"), |b| {
            let mut metronome = black_box(bats_lib::metronome::Metronome::new(
                DEFAULT_SAMPLE_RATE,
                120.0,
            ));
            b.iter(move || {
                for _ in 0..buffer_size {
                    metronome.next_position();
                }
            })
        });
    }
}

fn toof_benchmark(c: &mut Criterion) {
    for buffer_size in BUFFER_SIZES {
        c.bench_function(&format!("toof_{buffer_size}"), |b| {
            let mut bats = bats_lib::Bats::new(DEFAULT_SAMPLE_RATE, buffer_size);
            let (mut left, mut right) = make_buffers(buffer_size);
            let midi = black_box([
                (0, PRESS_C4.clone()),
                (2 * buffer_size as u32 / 4, PRESS_A4.clone()),
                (3 * buffer_size as u32 / 4, RELEASE_C4.clone()),
                ((4 * buffer_size as u32 - 1) / 4, RELEASE_A4.clone()),
            ]);
            let midi_ref = black_box(&midi);
            b.iter(move || {
                bats.process(midi_ref, &mut left, &mut right);
            })
        });
    }
}

criterion_group!(benches, bats_benchmark, metronome_benchmark, toof_benchmark);
criterion_main!(benches);

fn make_buffers(buffer_size: usize) -> (Vec<f32>, Vec<f32>) {
    black_box((vec![0f32; buffer_size], vec![0f32; buffer_size]))
}
