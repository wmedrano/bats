use bats_lib::plugin::{toof::Toof, BatsInstrument};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wmidi::{Channel, MidiMessage, Note, U7};

const BUFFER_SIZE: usize = 128;
const SAMPLE_RATE: f32 = 44100.0;
const PRESS_C4: MidiMessage<'static> = MidiMessage::NoteOn(Channel::Ch1, Note::C4, U7::MAX);
const RELEASE_C4: MidiMessage<'static> = MidiMessage::NoteOff(Channel::Ch1, Note::C4, U7::MIN);
const PRESS_A4: MidiMessage<'static> = MidiMessage::NoteOn(Channel::Ch1, Note::A4, U7::MAX);
const RELEASE_A4: MidiMessage<'static> = MidiMessage::NoteOff(Channel::Ch1, Note::A4, U7::MIN);

fn bats_benchmark(c: &mut Criterion) {
    c.bench_function(&format!("bats_init"), |b| {
        b.iter(|| {
            let _ = bats_lib::Bats::new(SAMPLE_RATE, BUFFER_SIZE);
        })
    });
    c.bench_function(&format!("bats_empty"), |b| {
        let mut bats = bats_lib::Bats::new(SAMPLE_RATE, BUFFER_SIZE);
        let (mut left, mut right) = make_buffers(BUFFER_SIZE);
        let midi = black_box(&[]);
        b.iter(move || {
            bats.process(midi, &mut left, &mut right);
        })
    });
    c.bench_function(&format!("bats_with_plugins"), |b| {
        let mut bats = black_box(bats_lib::Bats::new(SAMPLE_RATE, BUFFER_SIZE));
        for _ in 0..8 {
            bats.add_plugin(Toof::new(SAMPLE_RATE));
        }
        let (mut left, mut right) = make_buffers(BUFFER_SIZE);
        let midi = black_box([
            (0, PRESS_C4.clone()),
            (BUFFER_SIZE as u32 / 2, RELEASE_C4.clone()),
        ]);
        let midi_ref = black_box(&midi);
        b.iter(move || {
            bats.process(midi_ref, &mut left, &mut right);
        })
    });
}

fn metronome_benchmark(c: &mut Criterion) {
    c.bench_function(&format!("metronome_new"), |b| {
        b.iter(move || {
            let mut metronome = black_box(bats_lib::metronome::Metronome::new(SAMPLE_RATE, 120.0));
            metronome.next_position()
        })
    });

    c.bench_function(&format!("metronome_tick"), |b| {
        let mut metronome = black_box(bats_lib::metronome::Metronome::new(SAMPLE_RATE, 120.0));
        b.iter(move || {
            for _ in 0..BUFFER_SIZE {
                metronome.next_position();
            }
        })
    });
}

fn toof_benchmark(c: &mut Criterion) {
    c.bench_function(&format!("toof_process"), |b| {
        let mut toof = black_box(Toof::new(SAMPLE_RATE));
        let (mut left, mut right) = make_buffers(BUFFER_SIZE);
        let midi = black_box([
            (0, PRESS_C4.clone()),
            (2 * BUFFER_SIZE as u32 / 4, PRESS_A4.clone()),
            (3 * BUFFER_SIZE as u32 / 4, RELEASE_C4.clone()),
            ((4 * BUFFER_SIZE as u32 - 1) / 4, RELEASE_A4.clone()),
        ]);
        let midi_ref = black_box(&midi);
        b.iter(move || {
            toof.process(midi_ref, &mut left, &mut right);
        })
    });
    c.bench_function(&format!("toof_process_no_filter"), |b| {
        let mut toof = black_box(Toof::new(SAMPLE_RATE));
        toof.bypass_filter = true;
        let (mut left, mut right) = make_buffers(BUFFER_SIZE);
        let midi = black_box([
            (0, PRESS_C4.clone()),
            (2 * BUFFER_SIZE as u32 / 4, PRESS_A4.clone()),
            (3 * BUFFER_SIZE as u32 / 4, RELEASE_C4.clone()),
            ((4 * BUFFER_SIZE as u32 - 1) / 4, RELEASE_A4.clone()),
        ]);
        let midi_ref = black_box(&midi);
        b.iter(move || {
            toof.process(midi_ref, &mut left, &mut right);
        })
    });
}

criterion_group!(benches, bats_benchmark, metronome_benchmark, toof_benchmark);
criterion_main!(benches);

fn make_buffers(buffer_size: usize) -> (Vec<f32>, Vec<f32>) {
    black_box((vec![0f32; buffer_size], vec![0f32; buffer_size]))
}
