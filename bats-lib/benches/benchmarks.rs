use bats_dsp::{buffers::Buffers, sample_rate::SampleRate};
use bats_lib::plugin::{toof::Toof, BatsInstrument, BatsInstrumentExt};
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
            let _ = bats_lib::Bats::new(SampleRate::new(SAMPLE_RATE), BUFFER_SIZE);
        })
    });
    c.bench_function(&format!("bats_empty"), |b| {
        let mut bats = bats_lib::Bats::new(SampleRate::new(SAMPLE_RATE), BUFFER_SIZE);
        let mut buffers = black_box(Buffers::new(BUFFER_SIZE));
        let midi = black_box(&[]);
        b.iter(move || {
            bats.process(midi, &mut buffers.left, &mut buffers.right);
        })
    });
    c.bench_function(&format!("bats_with_8_toofs"), |b| {
        let mut bats = black_box(bats_lib::Bats::new(
            SampleRate::new(SAMPLE_RATE),
            BUFFER_SIZE,
        ));
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
    c.bench_function(&format!("transport_new"), |b| {
        b.iter(move || {
            black_box(bats_lib::transport::Transport::new(
                SampleRate::new(SAMPLE_RATE),
                BUFFER_SIZE,
                120.0,
            ))
        })
    });

    c.bench_function(&format!("transport_tick"), |b| {
        let mut transport = black_box(bats_lib::transport::Transport::new(
            SampleRate::new(SAMPLE_RATE),
            BUFFER_SIZE,
            120.0,
        ));
        let mut buffers = Buffers::new(BUFFER_SIZE);
        b.iter(move || {
            for _ in 0..BUFFER_SIZE {
                transport.process(&mut buffers.left, &mut buffers.right);
            }
        })
    });
}

fn toof_benchmark(c: &mut Criterion) {
    c.bench_function(&format!("toof_process"), |b| {
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
            toof.process_batch(midi_ref.iter().map(|(a, b)| (*a, b)), &mut buffers);
        })
    });
    c.bench_function(&format!("toof_process_no_filter"), |b| {
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
            toof.process_batch(midi_ref.iter().map(|(a, b)| (*a, b)), &mut buffers);
        })
    });
}

criterion_group!(benches, bats_benchmark, transport_benchmark, toof_benchmark);
criterion_main!(benches);
