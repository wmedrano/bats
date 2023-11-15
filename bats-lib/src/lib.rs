use bats_dsp::{buffers::Buffers, sample_rate::SampleRate};
use plugin::{toof::Toof, BatsInstrument, MidiEvent};
use transport::Transport;
use wmidi::MidiMessage;

pub mod plugin;
pub mod transport;

/// Handles all processing.
#[derive(Debug)]
pub struct Bats {
    /// The transport.
    pub transport: Transport,
    /// The id of the track that should take user midi input.
    pub armed_track: usize,
    /// The sample rate.
    pub sample_rate: SampleRate,
    /// The buffer size.
    pub buffer_size: usize,
    /// Temporary buffer for midi data.
    pub midi_buffer: Vec<(u32, MidiMessage<'static>)>,
    /// The tracks.
    pub tracks: [Track; Bats::SUPPORTED_TRACKS],
}

/// An plugin with output buffers.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    /// The plugin.
    pub plugin: Option<Box<Toof>>,
    /// The track volume.
    pub volume: f32,
    /// The buffers to output data to.
    pub output: Buffers,
    /// The midi sequence to play.
    pub sequence: Vec<MidiEvent>,
}

impl Track {
    /// Create a new track.
    pub fn new(buffer_size: usize) -> Track {
        Track {
            plugin: None,
            volume: 1.0,
            output: Buffers::new(buffer_size),
            // TODO: Determine the right capacity for sequences.
            sequence: Vec::with_capacity(1024),
        }
    }
}

impl Bats {
    /// The number of supported tracks.
    pub const SUPPORTED_TRACKS: usize = 8;

    /// Create a new `Bats` object.
    pub fn new(sample_rate: SampleRate, buffer_size: usize) -> Bats {
        Bats {
            transport: Transport::new(sample_rate, buffer_size, 120.0),
            armed_track: 0,
            sample_rate,
            buffer_size,
            // TODO: Determine proper capacity.
            midi_buffer: Vec::with_capacity(4096),
            tracks: core::array::from_fn(|_| Track::new(buffer_size)),
        }
    }

    /// Process midi data and output audio.
    pub fn process(
        &mut self,
        midi: &[(u32, MidiMessage<'static>)],
        left: &mut [f32],
        right: &mut [f32],
    ) {
        self.transport.process(left, right);
        for (id, track) in self.tracks.iter_mut().enumerate() {
            if id == self.armed_track {
                self.transport
                    .push_to_sequence(&mut track.sequence, midi.iter());
            }
            self.transport
                .sequence_to_frames(&mut self.midi_buffer, &track.sequence);
            if let Some(p) = track.plugin.as_mut() {
                let midi_in = self.midi_buffer.iter().map(|(a, b)| (*a, b));
                p.process_batch(midi_in, &mut track.output);
            }
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
        midi: &[(u32, wmidi::MidiMessage<'static>)],
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

    use wmidi::{Channel, Note, U7};

    use super::*;

    fn to_has_signal_vec(s: &[f32]) -> Vec<bool> {
        s.iter().map(|v| v.abs() > f32::EPSILON).collect()
    }

    #[test]
    fn bats_implements_debug() {
        let b = Bats::new(SampleRate::new(44100.0), 1024);
        let _: &dyn std::fmt::Debug = &b;
    }

    #[test]
    fn bats_has_right_number_of_tracks() {
        let b = Bats::new(SampleRate::new(44100.0), 1024);
        assert_eq!(b.tracks.len(), Bats::SUPPORTED_TRACKS);
    }

    #[test]
    fn no_input_produces_silence() {
        let mut left = [1.0, 2.0, 3.0];
        let mut right = [4.0, 5.0, 6.0];
        let mut b = Bats::new(SampleRate::new(44100.0), left.len());
        b.process(&[], &mut left, &mut right);
        assert_eq!(left, [0.0, 0.0, 0.0]);
        assert_eq!(right, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn no_input_with_transport_produces_metronome_sound() {
        let mut left = [1.0, 2.0, 3.0];
        let mut right = [4.0, 5.0, 6.0];
        let sample_rate = SampleRate::new(16.0);
        let mut b = Bats::new(sample_rate, left.len());
        b.transport.set_synth_decay(sample_rate, 0.0);
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
    fn metronome_ticks_regularly() {
        let mut buffers = Buffers::new(44100);
        let mut bats = Bats::new(SampleRate::new(44100.0), 44100);
        bats.transport.metronome_volume = 1.0;
        bats.transport
            .set_synth_decay(SampleRate::new(44100.0), 0.0);
        bats.transport.set_bpm(SampleRate::new(44100.0), 120.0);
        bats.process(&[], &mut buffers.left, &mut buffers.right);
        // At 120 BPM, it should tick twice in a second.
        assert_eq!(buffers.left.iter().filter(|v| 0.0 != **v).count(), 2);
        assert_eq!(buffers.right.iter().filter(|v| 0.0 != **v).count(), 2);
    }

    #[test]
    fn midi_without_arm_remains_silent() {
        let sample_count = 3;
        let mut b = Bats::new(SampleRate::new(44100.0), sample_count);
        b.tracks[0] = Track {
            plugin: Some(Toof::new(SampleRate::new(44100.0))),
            volume: 1.0,
            output: Buffers::new(sample_count),
            sequence: Vec::new(),
        };
        b.armed_track = 100;
        let buffers = b.process_to_buffer(
            sample_count,
            &[(
                0,
                wmidi::MidiMessage::NoteOn(Channel::Ch1, Note::C3, U7::MAX),
            )],
        );
        assert_eq!(
            buffers,
            Buffers {
                left: vec![0.0, 0.0, 0.0],
                right: vec![0.0, 0.0, 0.0]
            }
        );
    }

    #[test]
    fn midi_and_armed_produces_sound() {
        let sample_count = 3;
        let mut b = Bats::new(SampleRate::new(44100.0), sample_count);
        b.tracks[0] = Track {
            plugin: Toof::new(SampleRate::new(44100.0)).into(),
            volume: 1.0,
            output: Buffers::new(sample_count),
            sequence: Vec::new(),
        };
        b.armed_track = 0;
        let buffers = b.process_to_buffer(
            sample_count,
            &[(
                0,
                wmidi::MidiMessage::NoteOn(Channel::Ch1, Note::C3, U7::MAX),
            )],
        );
        assert_ne!(
            buffers,
            Buffers {
                left: vec![0.0, 0.0, 0.0],
                right: vec![0.0, 0.0, 0.0]
            }
        );
    }
}
