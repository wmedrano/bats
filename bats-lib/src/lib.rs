use bats_dsp::{buffers::Buffers, sample_rate::SampleRate};
use bmidi::MidiMessage;

use track::{Track, TrackProcessContext};
use transport::Transport;

pub mod builder;
pub mod plugin;
pub mod track;
pub mod transport;

/// Handles all processing.
#[derive(Clone, Debug, PartialEq)]
pub struct Bats {
    /// The transport.
    pub transport: Transport,
    /// The id of the track that should take user midi input.
    pub armed_track: usize,
    /// True if recording to sequence is enabled.
    pub recording_enabled: bool,
    /// The sample rate.
    pub sample_rate: SampleRate,
    /// The buffer size.
    pub buffer_size: usize,
    /// Temporary buffer for midi data.
    pub midi_buffer: Vec<(u32, MidiMessage)>,
    /// The tracks.
    pub tracks: [Track; Bats::SUPPORTED_TRACKS],
}

impl Bats {
    /// The number of supported tracks.
    pub const SUPPORTED_TRACKS: usize = 8;

    /// Process midi data and output audio.
    pub fn process(&mut self, midi: &[(u32, MidiMessage)], left: &mut [f32], right: &mut [f32]) {
        self.transport.process(left, right);
        for (id, track) in self.tracks.iter_mut().enumerate() {
            let is_armed = id == self.armed_track;
            let midi_in = if is_armed { midi } else { &[] };
            track.process(TrackProcessContext {
                record_to_sequence: self.recording_enabled,
                transport: &self.transport,
                midi_in,
                tmp_midi_buffer: &mut self.midi_buffer,
            });
            mix(left, &track.output.left, track.volume);
            mix(right, &track.output.right, track.volume);
        }
    }

    /// Run `process` but output the results to a new `Buffers` object.
    ///
    /// Implemented for convenience but performance critical applications should preallocate buffers
    /// and call `process`.
    pub fn process_to_buffer(
        &mut self,
        sample_count: usize,
        midi: &[(u32, MidiMessage)],
    ) -> Buffers {
        let mut buffers = Buffers::new(sample_count);
        self.process(midi, &mut buffers.left, &mut buffers.right);
        buffers
    }
}

/// Mix `src` onto `dst` weighted by `volume`.
fn mix(dst: &mut [f32], src: &[f32], volume: f32) {
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        *d += volume * s;
    }
}

#[cfg(test)]
mod tests {

    use bmidi::{Channel, Note, U7};

    use crate::{builder::BatsBuilder, plugin::toof::Toof};

    use super::*;

    fn to_has_signal_vec(s: &[f32]) -> Vec<bool> {
        s.iter().map(|v| v.abs() > f32::EPSILON).collect()
    }

    #[test]
    fn bats_implements_debug() {
        let b = BatsBuilder {
            sample_rate: SampleRate::new(44100.0),
            buffer_size: 16,
            bpm: 120.0,
            tracks: Default::default(),
        }
        .build();
        let _: &dyn std::fmt::Debug = &b;
    }

    #[test]
    fn bats_has_right_number_of_tracks() {
        let b = BatsBuilder {
            sample_rate: SampleRate::new(44100.0),
            buffer_size: 1024,
            bpm: 120.0,
            tracks: Default::default(),
        }
        .build();
        assert_eq!(b.tracks.len(), Bats::SUPPORTED_TRACKS);
    }

    #[test]
    fn no_input_produces_silence() {
        let mut buffers = Buffers {
            left: vec![1.0, 2.0, 3.0],
            right: vec![4.0, 5.0, 6.0],
        };
        assert!(!buffers.is_zero());
        let mut b = BatsBuilder {
            sample_rate: SampleRate::new(44100.0),
            buffer_size: buffers.len(),
            bpm: 120.0,
            tracks: Default::default(),
        }
        .build();
        b.process(&[], &mut buffers.left, &mut buffers.right);
        assert!(buffers.is_zero());
    }

    #[test]
    fn no_input_with_transport_produces_metronome_sound() {
        let mut left = [1.0, 2.0, 3.0];
        let mut right = [4.0, 5.0, 6.0];
        let mut b = BatsBuilder {
            sample_rate: SampleRate::new(16.0),
            buffer_size: left.len(),
            bpm: 120.0,
            tracks: Default::default(),
        }
        .build();
        b.transport.set_synth_decay(SampleRate::new(16.0), 0.0);
        b.transport.metronome_volume = 1.0;
        b.process(&[], &mut left, &mut right);
        assert_eq!(
            to_has_signal_vec(&left),
            vec![true, false, false],
            "{left:?}"
        );
        assert_eq!(
            to_has_signal_vec(&right),
            vec![true, false, false],
            "{right:?}"
        );
    }

    #[test]
    fn midi_without_arm_remains_silent() {
        let sample_count = 3;
        let mut b = BatsBuilder {
            sample_rate: SampleRate::new(44100.0),
            buffer_size: sample_count,
            bpm: 120.0,
            tracks: Default::default(),
        }
        .build();
        b.tracks[0] = Track {
            plugin: Toof::new(SampleRate::new(44100.0)).into(),
            volume: 1.0,
            output: Buffers::new(sample_count),
            sequence: Vec::new(),
        };
        b.armed_track = 100;
        let buffers = b.process_to_buffer(
            sample_count,
            &[(0, MidiMessage::NoteOn(Channel::Ch1, Note::C3, U7::MAX))],
        );
        assert!(buffers.is_zero());
    }

    #[test]
    fn midi_and_armed_produces_sound() {
        let sample_count = 3;
        let mut b = BatsBuilder {
            sample_rate: SampleRate::new(44100.0),
            buffer_size: sample_count,
            bpm: 120.0,
            tracks: Default::default(),
        }
        .build();
        b.tracks[0] = Track {
            plugin: Toof::new(SampleRate::new(44100.0)).into(),
            volume: 1.0,
            output: Buffers::new(sample_count),
            sequence: Vec::new(),
        };
        b.armed_track = 0;
        let buffers = b.process_to_buffer(
            sample_count,
            &[(0, MidiMessage::NoteOn(Channel::Ch1, Note::C3, U7::MAX))],
        );
        assert!(!buffers.is_zero());
    }
}
