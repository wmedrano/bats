use std::ops::Range;

use bats_dsp::{position::Position, sample_rate::SampleRate, sawtooth::Sawtooth};
use wmidi::{Channel, MidiMessage, Note, U7};

use crate::plugin::BatsInstrument;

/// Tracks position according to the specified BPM.
#[derive(Clone, Debug, PartialEq)]
pub struct Transport {
    /// The volume of the metronome.
    pub metronome_volume: f32,
    /// The positions for each frame.
    transport: Vec<Position>,
    /// The beats per minute of the transport.
    bpm: f32,
    /// The current position fo the transport.
    position: Position,
    /// The amount of advancement the transport undergoes per frame.
    position_per_sample: Position,
    /// The metronome synth.
    sound_gen: MetronomeSynth,
}

impl Transport {
    /// Create a new transport with the given sample rate and beats per minute.
    pub fn new(sample_rate: SampleRate, buffer_size: usize, bpm: f32) -> Transport {
        Transport {
            metronome_volume: 0.0,
            transport: Vec::with_capacity(buffer_size + 1),
            bpm,
            position: Position::default(),
            position_per_sample: Position::delta_from_bpm(sample_rate, bpm),
            sound_gen: MetronomeSynth::new(sample_rate),
        }
    }

    /// Create a new transport and populate the transport values. Equivalent to creating a new
    /// `Transport` and calling `process` with a buffer of size `buffer_size`.
    pub fn new_prepopulated(sample_rate: SampleRate, buffer_size: usize, bpm: f32) -> Transport {
        let mut t = Transport::new(sample_rate, buffer_size, bpm);
        t.populate_transport(buffer_size);
        t
    }

    /// Set the beats per minute for a metronome.
    pub fn set_bpm(&mut self, sample_rate: SampleRate, bpm: f32) {
        self.bpm = bpm;
        self.position_per_sample = Position::delta_from_bpm(sample_rate, bpm);
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
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let samples = left.len().min(right.len());
        self.populate_transport(samples);
        self.populate_metronome_sound(left, right);
    }

    /// Populate the transport with `samples + 1` values. The first element of `transport` will
    /// always be the last element of the previous value. If there is no previous value then
    /// `Position::MAX` will be used.
    fn populate_transport(&mut self, samples: usize) {
        self.transport.clear();
        self.transport.extend((0..samples).map(|_| {
            let ret = self.position;
            self.position += self.position_per_sample;
            if self.position.beat() >= 16 {
                self.position.set_beat(self.position.beat() % 16);
            }
            ret
        }));
        self.transport.push(self.position);
        debug_assert!(
            self.transport.len() == samples + 1,
            "{} == {} + 1",
            self.transport.len(),
            samples
        );
    }

    /// Iterate over all values in the transport.
    pub fn iter_transport(&self) -> impl '_ + Iterator<Item = Range<Position>> {
        self.transport.windows(2).map(|rng| match rng {
            [a, b] => *a..*b,
            _ => unreachable!(),
        })
    }

    /// Get the range for the given frame.
    pub fn range_for_frame(&self, frame: u32) -> Range<Position> {
        self.transport[frame as usize]..self.transport[(frame + 1) as usize]
    }

    /// Populate `left` and `right` by playing the metronome synth based on the beats in
    /// `transport`.
    fn populate_metronome_sound(&mut self, left: &mut [f32], right: &mut [f32]) {
        let default_note = MidiMessage::NoteOn(Channel::Ch1, Note::C4, U7::MAX);
        let new_measure_note = MidiMessage::NoteOn(Channel::Ch1, Note::C5, U7::MAX);
        let loop_note = MidiMessage::NoteOn(Channel::Ch1, Note::G5, U7::MAX);
        for (idx, pos) in {
            let transport: &[Position] = &self.transport;
            transport.windows(2).map(|rng| match rng {
                [a, b] => (*a, *b),
                _ => unreachable!(),
            })
        }
        .enumerate()
        {
            if pos.0.beat() != pos.1.beat() || pos.0 == Position::MIN {
                let note = match pos.1.beat() {
                    0 => &loop_note,
                    b if b % 4 == 0 => &new_measure_note,
                    _ => &default_note,
                };
                self.sound_gen.handle_midi(note);
            }
            let (v, _) = self.sound_gen.process();
            left[idx] = v * self.metronome_volume;
            right[idx] = v * self.metronome_volume;
        }
    }
}

/// A simple synthesize for the metronome.
#[derive(Copy, Clone, Debug, PartialEq)]
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

    fn handle_midi(&mut self, msg: &MidiMessage) {
        if let MidiMessage::NoteOn(_, n, _) = msg {
            self.wave = Sawtooth::new(self.sample_rate, n.to_freq_f32());
            self.amp = 1.0;
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
    use bats_dsp::buffers::Buffers;

    use super::*;

    #[test]
    fn transport_produces_beat_at_proper_time() {
        let bpm = 4.0 * 60.0; // 4 beats per second.
        let mut m = Transport::new(SampleRate::new(16.0), 10, bpm);
        let mut buffers = Buffers::new(10);
        m.process(&mut buffers.left, &mut buffers.right);
        assert_eq!(
            m.transport.clone(),
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
                Position::new(2.25),
                Position::new(2.5),
            ]
        );
    }

    #[test]
    fn metronome_ticks_regularly() {
        let mut buffers = Buffers::new(44100);
        // At 120 BPM, it should tick twice in a second. A second is 44100 samples.
        let mut transport = Transport::new(SampleRate::new(44100.0), 44100, 120.0);
        transport.metronome_volume = 1.0;
        transport.set_synth_decay(SampleRate::new(44100.0), 0.0);
        transport.process(&mut buffers.left, &mut buffers.right);
        assert_eq!(buffers.left.iter().filter(|v| 0.0 != **v).count(), 2);
        assert_eq!(buffers.right.iter().filter(|v| 0.0 != **v).count(), 2);
    }
}
