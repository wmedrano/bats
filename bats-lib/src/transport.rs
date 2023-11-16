use bats_dsp::{position::Position, sample_rate::SampleRate, sawtooth::Sawtooth};
use wmidi::{Channel, MidiMessage, Note, U7};

use crate::plugin::{BatsInstrument, MidiEvent};

/// Tracks position according to the specified BPM.
#[derive(Clone, Debug)]
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

    /// Push the frame timed items from `midi_iter` to the sequence `dst`.
    pub fn push_to_sequence<'a>(
        &self,
        dst: &mut Vec<MidiEvent>,
        midi_iter: impl 'a + Iterator<Item = &'a (u32, MidiMessage<'static>)>,
    ) {
        for (frame, midi) in midi_iter {
            let position = self.transport[*frame as usize + 1];
            dst.push(MidiEvent {
                position,
                midi: midi.clone(),
            });
        }
        dst.sort_by_key(|e| e.position);
    }

    /// Push the events in `sequence` onto `dst`. The events are paired (and sorted) with the frame
    /// they appear in.
    pub fn sequence_to_frames(
        &self,
        dst: &mut Vec<(u32, MidiMessage<'static>)>,
        sequence: &[MidiEvent],
    ) {
        if sequence.is_empty() {
            return;
        }
        let initial_len = dst.len();
        let placeholder_event = MidiEvent {
            position: Position::MAX,
            midi: MidiMessage::Reserved(0),
        };
        let transport_start = self.transport[0];
        // TODO: Use binary search for performance improvement.
        let start = sequence
            .iter()
            .position(|e| e.position >= transport_start)
            .unwrap_or(sequence.len());
        let mut sequence_iter = sequence
            .iter()
            .chain(std::iter::once(&placeholder_event))
            .cycle()
            .skip(start)
            .peekable();
        for (frame, (left, right)) in Self::iter_transport(&self.transport).enumerate() {
            let is_in_range = |event: &&MidiEvent| {
                if left < right {
                    (left..right).contains(&event.position)
                } else {
                    !(right..left).contains(&event.position)
                }
            };
            let mut has_looped = false;
            while let Some(event) = sequence_iter.next_if(is_in_range) {
                if event != &placeholder_event {
                    dst.push((frame as u32, event.midi.clone()));
                } else if has_looped {
                    // Only allow wrapping over once per position range.
                    continue;
                } else {
                    has_looped = true;
                }
            }
        }
        if initial_len != 0 && dst.len() != initial_len {
            dst.sort_by_key(|(frame, _)| *frame)
        }
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
        let previous = self.transport.pop().unwrap_or(Position::MAX);
        self.transport.clear();
        self.transport.extend(
            std::iter::once(previous).chain(
                std::iter::repeat_with(|| {
                    let ret = self.position;
                    self.position += self.position_per_sample;
                    if self.position.beat() >= 16 {
                        self.position.set_beat(self.position.beat() % 16);
                    }
                    ret
                })
                .take(samples),
            ),
        );
        debug_assert!(
            self.transport.len() == samples + 1,
            "{} == {} + 1",
            self.transport.len(),
            samples
        );
    }

    fn iter_transport(transport: &[Position]) -> impl '_ + Iterator<Item = (Position, Position)> {
        transport.windows(2).map(|rng| match rng {
            [a, b] => (*a, *b),
            _ => unreachable!(),
        })
    }

    /// Populate `left` and `right` by playing the metronome synth based on the beats in
    /// `transport`.
    fn populate_metronome_sound(&mut self, left: &mut [f32], right: &mut [f32]) {
        let default_note = wmidi::MidiMessage::NoteOn(Channel::Ch1, Note::C4, U7::MAX);
        let new_measure_note = wmidi::MidiMessage::NoteOn(Channel::Ch1, Note::C5, U7::MAX);
        let loop_note = wmidi::MidiMessage::NoteOn(Channel::Ch1, Note::G5, U7::MAX);
        for (idx, pos) in Self::iter_transport(&self.transport).enumerate() {
            if pos.0.beat() != pos.1.beat() {
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
        if let wmidi::MidiMessage::NoteOn(_, n, _) = msg {
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
    fn metronome_produces_beat_at_proper_time() {
        let bpm = 4.0 * 60.0; // 4 beats per second.
        let mut m = Transport::new(SampleRate::new(16.0), 10, bpm);
        let mut buffers = Buffers::new(10);
        m.process(&mut buffers.left, &mut buffers.right);
        assert_eq!(
            m.transport.clone(),
            vec![
                Position::MAX,
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
