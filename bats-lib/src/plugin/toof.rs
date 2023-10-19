use wmidi::{MidiMessage, Note, U7};

use super::BatsInstrument;

/// A simple Sawtooth plugin.
#[derive(Debug, Clone)]
pub struct Toof {
    /// The number of seconds per sample. This is `1.0 / sample_rate`.
    seconds_per_sample: f32,
    /// The active voices for toof.
    voices: Vec<ToofVoice>,
}

/// A single voice for the Toof plugin. Each voice contains a single
/// note.
#[derive(Clone, Debug)]
struct ToofVoice {
    /// The midi note for the voice.
    note: Note,
    /// The amplitude delta per sample. This is proportional to the
    /// frequency.
    amplitude_per_sample: f32,
    /// The current amplitude.
    amplitude: f32,
}

impl BatsInstrument for Toof {
    /// Create a new Toof plugin with the given sample rate.
    fn new(sample_rate: f32) -> Toof {
        Toof {
            seconds_per_sample: 1.0 / sample_rate,
            voices: Vec::with_capacity(128),
        }
    }

    /// Handle midi processing and output audio signal to `left_out`
    /// and `right_out`.
    fn process<'a>(
        &mut self,
        midi_in: &[(u32, MidiMessage<'static>)],
        left_out: &mut [f32],
        right_out: &mut [f32],
    ) {
        self.process_mono(midi_in, left_out);
        right_out.copy_from_slice(left_out);
    }
}

impl Toof {
    /// Handle the processing and output to a single audio output.
    fn process_mono(&mut self, midi_in: &[(u32, MidiMessage<'static>)], out: &mut [f32]) {
        let mut midi_in = midi_in.iter().peekable();
        for (idx, out) in out.iter_mut().enumerate() {
            while let Some((_, msg)) = midi_in.next_if(|(frame, _)| *frame <= idx as u32) {
                self.handle_midi(msg);
            }
            *out = self.voices.iter_mut().map(|v| v.next_sample()).sum();
        }
    }

    /// Handle a midi event.
    fn handle_midi(&mut self, msg: &MidiMessage) {
        match msg {
            MidiMessage::NoteOff(_, note, _) | MidiMessage::NoteOn(_, note, U7::MIN) => {
                self.voices.retain(|v| v.note != *note);
            }
            MidiMessage::NoteOn(_, note, _) => {
                self.voices
                    .push(ToofVoice::new(self.seconds_per_sample, *note));
            }
            _ => (),
        }
    }
}

impl ToofVoice {
    /// Create a new Toof voice.
    fn new(seconds_per_sample: f32, note: Note) -> ToofVoice {
        let amplitude_per_cycle = 2.0;
        let cycles_per_second = note.to_freq_f32();
        let amplitude_per_sample = amplitude_per_cycle * cycles_per_second * seconds_per_sample;
        ToofVoice {
            note,
            amplitude_per_sample,
            amplitude: 0.0,
        }
    }

    /// Get the next sample value for this Toof voice.
    fn next_sample(&mut self) -> f32 {
        let ret = self.amplitude;
        self.amplitude += self.amplitude_per_sample;
        if self.amplitude > 1.0 {
            self.amplitude -= 2.0;
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use wmidi::{Channel, MidiMessage, Note, U7};

    use super::*;

    #[test]
    fn note_press_produces_audio() {
        let mut s = Toof::new(44100.0);
        let (left, right) = s.process_to_vec(44100, &[]);
        assert_eq!(left, vec![0f32; 44100]);
        assert_eq!(right, vec![0f32; 44100]);

        let (left, right) = s.process_to_vec(
            44100,
            &[(0, MidiMessage::NoteOn(Channel::Ch1, Note::C3, U7::MAX))],
        );
        assert_ne!(left, vec![0f32; 44100]);
        assert_ne!(right, vec![0f32; 44100]);
    }

    #[test]
    fn key_presses_produce_polyphonic_sound() {
        let note_a = (0, MidiMessage::NoteOn(Channel::Ch1, Note::A3, U7::MAX));
        let note_b = (0, MidiMessage::NoteOn(Channel::Ch1, Note::B4, U7::MAX));
        let (signal_a_left, signal_a_right) =
            Toof::new(44100.0).process_to_vec(100, &[note_a.clone()]);
        let (signal_b_left, signal_b_right) =
            Toof::new(44100.0).process_to_vec(100, &[note_b.clone()]);
        let (signal_summed_left, signal_summed_right) =
            Toof::new(44100.0).process_to_vec(100, &[note_a, note_b]);
        assert_eq!(
            signal_summed_left,
            signal_a_left
                .iter()
                .zip(signal_b_left.iter())
                .map(|(a, b)| *a + *b)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            signal_summed_right,
            signal_a_right
                .iter()
                .zip(signal_b_right.iter())
                .map(|(a, b)| *a + *b)
                .collect::<Vec<_>>()
        );
    }
}
