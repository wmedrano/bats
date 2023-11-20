#[macro_use]
extern crate criterion;

use criterion::{black_box, Criterion};
use std::{convert::TryFrom, time::Duration};

const MESSAGES: [bmidi::MidiMessage; 19] = [
    bmidi::MidiMessage::NoteOn(bmidi::Channel::Ch1, bmidi::Note::C3, bmidi::U7::MAX),
    bmidi::MidiMessage::NoteOff(bmidi::Channel::Ch2, bmidi::Note::A3, bmidi::U7::MIN),
    bmidi::MidiMessage::PolyphonicKeyPressure(bmidi::Channel::Ch3, bmidi::Note::B1, bmidi::U7::MAX),
    bmidi::MidiMessage::ControlChange(
        bmidi::Channel::Ch4,
        bmidi::ControlFunction::DAMPER_PEDAL,
        bmidi::U7::MAX,
    ),
    bmidi::MidiMessage::ProgramChange(bmidi::Channel::Ch5, bmidi::U7::MIN),
    bmidi::MidiMessage::ChannelPressure(bmidi::Channel::Ch6, bmidi::U7::MAX),
    bmidi::MidiMessage::PitchBendChange(bmidi::Channel::Ch7, bmidi::U14::MAX),
    bmidi::MidiMessage::Start,
    bmidi::MidiMessage::SysEx,
    bmidi::MidiMessage::MidiTimeCode(bmidi::U7::MAX),
    bmidi::MidiMessage::SongPositionPointer(bmidi::U14::MIN),
    bmidi::MidiMessage::SongSelect(bmidi::U7::MIN),
    bmidi::MidiMessage::TuneRequest,
    bmidi::MidiMessage::TimingClock,
    bmidi::MidiMessage::Start,
    bmidi::MidiMessage::Continue,
    bmidi::MidiMessage::Stop,
    bmidi::MidiMessage::ActiveSensing,
    bmidi::MidiMessage::Reset,
];

fn bench_from_bytes(c: &mut Criterion) {
    let bytes = {
        let mut bytes = vec![0u8; MESSAGES.iter().map(|m| m.bytes_size()).sum()];
        let mut start = 0;
        for message in MESSAGES.iter() {
            let end = start
                + message
                    .copy_to_slice(&mut bytes.as_mut_slice()[start..])
                    .unwrap();
            start = end;
        }
        bytes
    };
    c.benchmark_group("bmidi")
        .measurement_time(Duration::from_secs(10))
        .confidence_level(0.99)
        .bench_function("MidiMessage::try_from", |b| {
            let bytes = black_box(bytes.clone());
            b.iter(|| {
                let mut messages: Vec<bmidi::MidiMessage> = Vec::with_capacity(MESSAGES.len());
                let mut start = 0;
                while start < bytes.len() {
                    let message = bmidi::MidiMessage::try_from(&bytes[start..]).unwrap();
                    start += message.bytes_size();
                    messages.push(message);
                }
                messages
            });
        });
}

criterion_group!(benchmarks, bench_from_bytes);

criterion_main!(benchmarks);
