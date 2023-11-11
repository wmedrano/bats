use bats_dsp::{sample_rate::SampleRate, sawtooth::Sawtooth};
use wmidi::{Channel, Note, U7};

use crate::{plugin::BatsInstrument, position::Position};

/// Tracks position according to the specified BPM.
#[derive(Clone, Debug)]
pub struct Metronome {
    /// The volume of the metronome.
    pub volume: f32,
    /// The beats per minute of the metronome.
    bpm: f32,
    /// The current position fo the transport.
    position: Position,
    /// The amount of advancement the transport undergoes per frame.
    position_per_sample: Position,
    /// The metronome synth.
    sound_gen: MetronomeSynth,
}

impl Metronome {
    /// Create a new metronome with the given sample rate and beats per minute.
    pub fn new(sample_rate: SampleRate, bpm: f32) -> Metronome {
        let mut m = Metronome {
            volume: 0.0,
            bpm,
            position: Position::default(),
            position_per_sample: Position::default(),
            sound_gen: MetronomeSynth::new(sample_rate),
        };
        m.set_bpm(sample_rate, bpm);
        m
    }

    /// Set the beats per minute for a metronome.
    pub fn set_bpm(&mut self, sample_rate: SampleRate, bpm: f32) {
        self.bpm = bpm;
        let beats_per_second = bpm / 60.0;
        self.position_per_sample =
            Position::new(beats_per_second as f64 * sample_rate.seconds_per_sample() as f64);
    }

    /// Get the next position from the metronome.
    pub fn next_position(&mut self) -> Position {
        let ret = self.position;
        self.position += self.position_per_sample;
        ret
    }

    /// Get the current bpm.
    pub fn bpm(&self) -> f32 {
        self.bpm
    }

    /// Set the decay of the synth.
    pub fn set_synth_decay(&mut self, sample_rate: SampleRate, duration_seconds: f32) {
        if duration_seconds <= 0.0 {
            self.sound_gen.amp_delta = -1.0;
            return;
        }
        let frames = duration_seconds / sample_rate.seconds_per_sample();
        self.sound_gen.amp_delta = -1.0 / frames;
    }

    /// Populate `transport` with the right position values. `left` and `right` are filled with the
    /// signal for the metronome synth.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], transport: &mut Vec<Position>) {
        let samples = left.len().min(right.len());
        self.populate_transport(samples, transport);
        self.populate_metronome_sound(left, right, transport);
    }

    /// Populate the transport with `samples + 1` values. The first element of `transport` will
    /// always be the last element of the previous value. If there is no previous value then
    /// `Position::MAX` will be used.
    fn populate_transport(&mut self, samples: usize, transport: &mut Vec<Position>) {
        let previous = transport.pop().unwrap_or(Position::MAX);
        transport.clear();
        transport.extend(
            std::iter::once(previous)
                .chain(std::iter::repeat_with(|| self.next_position()).take(samples)),
        );
        debug_assert!(
            transport.len() == samples + 1,
            "{} == {} + 1",
            transport.len(),
            samples
        );
    }

    /// Populate `left` and `right` by playing the metronome synth based on the beats in
    /// `transport`.
    fn populate_metronome_sound(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        transport: &[Position],
    ) {
        let low = wmidi::MidiMessage::NoteOn(Channel::Ch1, Note::C5, U7::MAX);
        let high = wmidi::MidiMessage::NoteOn(Channel::Ch1, Note::C6, U7::MAX);
        for (idx, pos) in transport.windows(2).enumerate() {
            match pos {
                [a, b] => {
                    if a.beat() != b.beat() {
                        let note = if b.beat() % 4 == 0 { &high } else { &low };
                        self.sound_gen.handle_midi(note);
                    }
                }
                _ => unreachable!(),
            }
            let (v, _) = self.sound_gen.process();
            left[idx] = v * self.volume;
            right[idx] = v * self.volume;
        }
    }
}

impl Iterator for Metronome {
    type Item = Position;

    fn next(&mut self) -> Option<Position> {
        Some(self.next_position())
    }
}

/// A simple synthesize for the metronome.
#[derive(Copy, Clone, Debug)]
struct MetronomeSynth {
    /// The sample rate.
    sample_rate: SampleRate,
    /// The current amp for the synth.
    amp: f32,
    /// The amount of delta (from decay) for the amp per frame.
    amp_delta: f32,
    /// The waveform for the synth.
    wave: Sawtooth,
}

impl MetronomeSynth {
    /// Create a new `MetronomeSynth`.
    fn new(sample_rate: SampleRate) -> MetronomeSynth {
        let duration_seconds = 0.1;
        let frames = duration_seconds / sample_rate.seconds_per_sample();
        MetronomeSynth {
            sample_rate,
            amp: 0.0,
            amp_delta: -1.0 / frames,
            wave: Sawtooth::new(sample_rate, 100.0),
        }
    }
}

impl BatsInstrument for MetronomeSynth {
    fn metadata(&self) -> &'static crate::plugin::metadata::Metadata {
        &crate::plugin::metadata::Metadata {
            name: "metronome_synth",
            params: &[],
        }
    }

    fn handle_midi(&mut self, msg: &wmidi::MidiMessage) {
        match msg {
            wmidi::MidiMessage::NoteOn(_, n, _) => {
                self.wave = Sawtooth::new(self.sample_rate, n.to_freq_f32());
                self.amp = 1.0;
            }
            _ => (),
        }
    }

    fn process(&mut self) -> (f32, f32) {
        if self.amp == 0.0 {
            return (0.0, 0.0);
        }
        let v = self.amp * self.wave.next_sample();
        self.amp += self.amp_delta;
        if self.amp < 0.0 {
            self.amp = 0.0;
        }
        (v, v)
    }

    fn param(&self, _id: u32) -> f32 {
        0.0
    }

    fn set_param(&mut self, _id: u32, _value: f32) {}

    fn batch_cleanup(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metronome_produces_beat_at_proper_time() {
        let bpm = 4.0 * 60.0; // 4 beats per second.
        let m = Metronome::new(SampleRate::new(16.0), bpm);
        assert_eq!(
            m.take(9).collect::<Vec<Position>>(),
            vec![
                Position::new(0.0),
                Position::new(0.25),
                Position::new(0.5),
                Position::new(0.75),
                Position::new(1.0),
                Position::new(1.25),
                Position::new(1.5),
                Position::new(1.75),
                Position::new(2.0),
            ]
        );
    }
}
