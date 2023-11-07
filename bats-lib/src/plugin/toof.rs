use arrayvec::ArrayVec;
use bats_dsp::{
    envelope::{Envelope, EnvelopeParams},
    moog_filter::MoogFilter,
    sample_rate::SampleRate,
    sawtooth::Sawtooth,
};
use wmidi::{MidiMessage, Note, U7};

use super::{
    metadata::{Param, ParamType},
    BatsInstrument, Metadata,
};

/// A simple Sawtooth plugin.
#[derive(Debug, Clone, PartialEq)]
pub struct Toof {
    /// If the filter is disabled.
    bypass_filter: bool,
    /// True if toof is polyphonic.
    is_polyphonic: bool,
    /// The sample rate.
    sample_rate: SampleRate,
    /// Parameters for envelope.
    envelope: EnvelopeParams,
    /// The low pass filter.
    filter: MoogFilter,
    /// The filter cutoff frequency.
    filter_cutoff: f32,
    /// The filter resonance.
    filter_resonance: f32,
    /// The active voices for toof.
    voices: ArrayVec<ToofVoice, 16>,
}

/// A single voice for the Toof plugin. Each voice contains a single
/// note.
#[derive(Copy, Clone, Debug, PartialEq)]
struct ToofVoice {
    /// The midi note for the voice.
    note: Note,
    /// The sawtooth wave.
    wave: Sawtooth,
    /// The envelope.
    envelope: Envelope,
}

impl Toof {
    /// Create a new Toof plugin with the given sample rate.
    pub fn new(sample_rate: SampleRate) -> Box<Toof> {
        let envelope = EnvelopeParams::new(sample_rate, 0.005, 0.08, 0.4, 0.05);
        Box::new(Toof {
            bypass_filter: false,
            is_polyphonic: false,
            sample_rate,
            envelope,
            filter: MoogFilter::new(sample_rate),
            filter_cutoff: MoogFilter::DEFAULT_FREQUENCY_CUTOFF,
            filter_resonance: MoogFilter::DEFAULT_RESONANCE,
            voices: ArrayVec::new(),
        })
    }
}

impl BatsInstrument for Toof {
    /// The name of the plugin.
    fn metadata(&self) -> &'static Metadata {
        &Metadata {
            name: "toof",
            params: &[
                Param {
                    id: 1,
                    name: "bypass filter",
                    param_type: ParamType::Bool,
                    default_value: 0.45,
                    min_value: 0.45,
                    max_value: 0.55,
                },
                Param {
                    id: 2,
                    name: "filter cutoff",
                    param_type: ParamType::Frequency,
                    default_value: MoogFilter::DEFAULT_FREQUENCY_CUTOFF,
                    min_value: 50.0,
                    max_value: 9000.0,
                },
                Param {
                    id: 3,
                    name: "filter resonance",
                    param_type: ParamType::Percent,
                    default_value: MoogFilter::DEFAULT_RESONANCE,
                    min_value: 0.01,
                    max_value: 0.70,
                },
                Param {
                    id: 4,
                    name: "polyphonic",
                    param_type: ParamType::Bool,
                    default_value: 0.45,
                    min_value: 0.45,
                    max_value: 0.55,
                },
            ],
        }
    }

    /// Handle the processing and output to a single audio output.
    fn process(&mut self) -> (f32, f32) {
        let v = self
            .voices
            .iter_mut()
            .map(|v| v.next_sample(&self.envelope))
            .sum();
        self.voices.retain(|v| v.envelope.is_active());
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
                for v in self.voices.iter_mut() {
                    if v.note == *note {
                        v.envelope.release(&self.envelope);
                    }
                }
            }
            MidiMessage::NoteOn(_, note, _) => {
                if self.is_polyphonic || self.voices.is_empty() {
                    if self.voices.is_full() {
                        self.voices.remove(0);
                    }
                    self.voices.push(ToofVoice::new(self.sample_rate, *note));
                } else {
                    self.voices[0].set_note(self.sample_rate, *note);
                }
            }
            MidiMessage::Reset => self.voices.clear(),
            _ => (),
        }
    }

    /// Get the value of a parameter.
    fn param(&self, id: u32) -> f32 {
        match id {
            1 => {
                if self.bypass_filter {
                    0.6
                } else {
                    0.4
                }
            }
            2 => self.filter_cutoff,
            3 => self.filter_resonance,
            4 => {
                if self.is_polyphonic {
                    0.6
                } else {
                    0.4
                }
            }
            _ => 0.0,
        }
    }

    /// Set a parameter.
    fn set_param(&mut self, id: u32, value: f32) {
        match id {
            1 => {
                self.bypass_filter = value >= 0.5;
            }
            2 => {
                self.filter_cutoff = value;
                self.filter
                    .set_cutoff(self.sample_rate, self.filter_cutoff, self.filter_resonance);
            }
            3 => {
                self.filter_resonance = value;
                self.filter
                    .set_cutoff(self.sample_rate, self.filter_cutoff, self.filter_resonance);
            }
            4 => {
                self.is_polyphonic = value >= 0.5;
            }
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
            envelope: Envelope::new(),
        }
    }

    /// Set a new note for the current voice.
    fn set_note(&mut self, sample_rate: SampleRate, note: Note) {
        self.note = note;
        self.wave.set_frequency(sample_rate, note.to_freq_f32());
        self.envelope = Envelope::new();
    }

    /// Retrieve the next sample.
    fn next_sample(&mut self, envelope: &EnvelopeParams) -> f32 {
        let wave_amp = self.wave.next_sample();
        let env_amp = self.envelope.next_sample(envelope);
        wave_amp * env_amp
    }
}

#[cfg(test)]
mod tests {
    use bats_dsp::buffers::Buffers;
    use wmidi::{Channel, MidiMessage, Note, U7};

    use crate::plugin::BatsInstrumentExt;

    use super::*;

    #[test]
    fn note_press_produces_audio() {
        let mut s = Toof::new(SampleRate::new(44100.0));
        let buffers = s.process_to_buffers(44100, &[]);
        assert_eq!(
            buffers,
            Buffers {
                left: vec![0f32; 44100],
                right: vec![0f32; 44100]
            }
        );

        let buffers = s.process_to_buffers(
            44100,
            &[(0, MidiMessage::NoteOn(Channel::Ch1, Note::C3, U7::MAX))],
        );
        assert_ne!(buffers.left, vec![0f32; 44100]);
        assert_ne!(buffers.right, vec![0f32; 44100]);
    }

    #[test]
    fn key_presses_produce_polyphonic_sound() {
        let note_a = (0, MidiMessage::NoteOn(Channel::Ch1, Note::A3, U7::MAX));
        let note_b = (0, MidiMessage::NoteOn(Channel::Ch1, Note::B4, U7::MAX));
        let mut toof = Toof::new(SampleRate::new(44100.0));
        toof.bypass_filter = true;
        toof.is_polyphonic = true;
        let signal_a = toof.clone().process_to_buffers(100, &[note_a.clone()]);
        let signal_b = toof.clone().process_to_buffers(100, &[note_b.clone()]);
        let signal_summed = toof.clone().process_to_buffers(100, &[note_a, note_b]);
        assert_eq!(
            signal_summed.left,
            signal_a
                .left
                .iter()
                .zip(signal_b.left.iter())
                .map(|(a, b)| *a + *b)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            signal_summed.right,
            signal_a
                .right
                .iter()
                .zip(signal_b.right.iter())
                .map(|(a, b)| *a + *b)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn clone_capacity_is_maintained() {
        let toof = Toof::new(SampleRate::new(44100.0));
        assert_eq!(toof.voices.capacity(), 16);
        assert_eq!(toof.clone().voices.capacity(), 16);
    }
}
