use bats_dsp::{buffers::Buffers, sample_rate::SampleRate};
use metronome::Metronome;
use plugin::{toof::Toof, BatsInstrument};
use position::Position;

pub mod metronome;
pub mod plugin;
pub mod position;

/// Handles all processing.
#[derive(Debug)]
pub struct Bats {
    /// The metronome.
    pub metronome: Metronome,
    /// The volume for the metronome.
    pub metronome_volume: f32,
    /// The positions for each sample.
    ///
    /// Note: The first entry in the slice represents the previous
    /// position.
    transport: Vec<Position>,
    /// The id of the track that should take user midi input.
    pub armed_track: usize,
    /// The sample rate.
    pub sample_rate: SampleRate,
    /// The buffer size.
    pub buffer_size: usize,
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
}

impl Track {
    /// Create a new track.
    pub fn new(buffer_size: usize) -> Track {
        Track {
            plugin: None,
            volume: 1.0,
            output: Buffers::new(buffer_size),
        }
    }
}

impl Bats {
    /// The number of supported tracks.
    pub const SUPPORTED_TRACKS: usize = 8;

    /// Create a new `Bats` object.
    pub fn new(sample_rate: SampleRate, buffer_size: usize) -> Bats {
        Bats {
            metronome: Metronome::new(sample_rate, 120.0),
            metronome_volume: 0.0,
            transport: Vec::with_capacity(buffer_size + 1),
            armed_track: 0,
            sample_rate,
            buffer_size,
            tracks: core::array::from_fn(|_| Track::new(buffer_size)),
        }
    }

    /// Process midi data and output audio.
    pub fn process(
        &mut self,
        midi: &[(u32, wmidi::MidiMessage<'static>)],
        left: &mut [f32],
        right: &mut [f32],
    ) {
        let sample_count = left.len().min(right.len());
        process_metronome(
            sample_count,
            &mut self.metronome,
            self.metronome_volume,
            left,
            right,
            &mut self.transport,
        );
        for (id, track) in self.tracks.iter_mut().enumerate() {
            let midi = if id == self.armed_track { midi } else { &[] };
            if let Some(p) = track.plugin.as_mut() {
                p.process_batch(midi, &mut track.output);
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

/// Process the metronome data. This produces the metronome sound and
/// updates the `transport` variable.
fn process_metronome(
    sample_count: usize,
    metronome: &mut Metronome,
    metronome_volume: f32,
    left: &mut [f32],
    right: &mut [f32],
    transport: &mut Vec<Position>,
) {
    left.fill(0.0);
    right.fill(0.0);
    let mut previous = match transport.pop() {
        Some(p) => p,
        None => {
            left[0] = metronome_volume;
            right[0] = metronome_volume;
            Position::default()
        }
    };
    transport.clear();
    transport.push(previous);
    for i in 0..sample_count {
        let next = metronome.next_position();
        if previous.beat() != next.beat() {
            left[i] = metronome_volume;
            right[i] = metronome_volume;
        }
        transport.push(next);
        previous = next;
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
    fn no_input_with_metronome_produces_metronome() {
        let mut left = [1.0, 2.0, 3.0];
        let mut right = [4.0, 5.0, 6.0];
        let mut b = Bats::new(SampleRate::new(44100.0), left.len());
        b.metronome_volume = 0.8;
        b.process(&[], &mut left, &mut right);
        assert_eq!(left, [0.8, 0.0, 0.0]);
        assert_eq!(right, [0.8, 0.0, 0.0]);
    }

    #[test]
    fn metronome_ticks_regularly() {
        let mut buffers = Buffers::new(44100);
        let mut bats = Bats::new(SampleRate::new(44100.0), 44100);
        bats.metronome_volume = 0.8;
        bats.metronome.set_bpm(SampleRate::new(44100.0), 120.0);
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
