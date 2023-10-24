use bats_dsp::{moog_filter::MoogFilter, sawtooth::Sawtooth, SampleRate};
use wmidi::{MidiMessage, Note, U7};

use super::BatsInstrument;

/// A simple Sawtooth plugin.
#[derive(Debug, Clone, PartialEq)]
pub struct Toof {
    /// If the filter is disabled.
    pub bypass_filter: bool,
    /// The sample rate.
    sample_rate: SampleRate,
    /// The active voices for toof.
    voices: Vec<ToofVoice>,
    /// The low pass filter.
    filter: MoogFilter,
}

/// A single voice for the Toof plugin. Each voice contains a single
/// note.
#[derive(Copy, Clone, Debug, PartialEq)]
struct ToofVoice {
    /// The midi note for the voice.
    note: Note,
    /// The sawtooth wave.
    wave: Sawtooth,
}

impl BatsInstrument for Toof {
    /// Create a new Toof plugin with the given sample rate.
    fn new(sample_rate: SampleRate) -> Toof {
        Toof {
            bypass_filter: false,
            sample_rate,
            voices: Vec::with_capacity(128),
            filter: MoogFilter::new(sample_rate),
        }
    }

    /// The name of the plugin.
    fn name(&self) -> &'static str {
        "toof"
    }

    /// Handle the processing and output to a single audio output.
    fn process(&mut self) -> (f32, f32) {
        let v = self.voices.iter_mut().map(|v| v.wave.next_sample()).sum();
        if self.bypass_filter {
            (v, v)
        } else {
            let v = self.filter.process(v);
            (v, v)
        }
    }

    /// Handle a midi event.
    #[cold]
    fn handle_midi(&mut self, msg: &MidiMessage) {
        match msg {
            MidiMessage::NoteOff(_, note, _) | MidiMessage::NoteOn(_, note, U7::MIN) => {
                self.voices.retain(|v| v.note != *note);
            }
            MidiMessage::NoteOn(_, note, _) => {
                self.voices.push(ToofVoice::new(self.sample_rate, *note));
            }
            MidiMessage::Reset => self.voices.clear(),
            _ => (),
        }
    }
}

impl ToofVoice {
    /// Create a new Toof voice.
    fn new(sample_rate: SampleRate, note: Note) -> ToofVoice {
        ToofVoice {
            note,
            wave: Sawtooth::new(sample_rate, note.to_freq_f32()),
        }
    }
}

#[cfg(test)]
mod tests {
    use wmidi::{Channel, MidiMessage, Note, U7};

    use super::*;

    #[test]
    fn note_press_produces_audio() {
        let mut s = Toof::new(SampleRate::new(44100.0));
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
        let mut toof = Toof::new(SampleRate::new(44100.0));
        toof.bypass_filter = true;
        let (signal_a_left, signal_a_right) = toof.clone().process_to_vec(100, &[note_a.clone()]);
        let (signal_b_left, signal_b_right) = toof.clone().process_to_vec(100, &[note_b.clone()]);
        let (signal_summed_left, signal_summed_right) =
            toof.clone().process_to_vec(100, &[note_a, note_b]);
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
