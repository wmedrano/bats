use bats_dsp::{buffers::Buffers, position::Position};
use wmidi::MidiMessage;

use crate::{
    plugin::{toof::Toof, BatsInstrument, MidiEvent},
    transport::Transport,
};

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

/// Context for processing a track.
#[derive(Debug)]
pub struct TrackProcessContext<'a> {
    /// If the contents of `midi_in` should be recorded to the track's sequence.
    pub record_to_sequence: bool,
    /// The transport for the buffer.
    pub transport: &'a Transport,
    /// The midi input.
    pub midi_in: &'a [(u32, MidiMessage<'static>)],
    /// Temporary midi buffer to use for scratch operations.
    pub tmp_midi_buffer: &'a mut Vec<(u32, MidiMessage<'static>)>,
}

impl Track {
    /// Create a new track.
    pub fn new(buffer_size: usize) -> Track {
        Track {
            plugin: None,
            volume: 1.0,
            output: Buffers::new(buffer_size),
            // TODO: Determine the right capacity for sequences.
            sequence: Vec::with_capacity(4096),
        }
    }

    /// Process the track. The resulting audio is updated in `self.output`.
    pub fn process(&mut self, ctx: TrackProcessContext) {
        ctx.tmp_midi_buffer.clear();
        self.sequence_to_midi_frames(ctx.tmp_midi_buffer, ctx.transport);
        if !ctx.midi_in.is_empty() {
            let should_sort = !ctx.tmp_midi_buffer.is_empty();
            ctx.tmp_midi_buffer.extend_from_slice(ctx.midi_in);
            if should_sort {
                ctx.tmp_midi_buffer.sort_by_key(|(frame, _)| *frame);
            }
            if ctx.record_to_sequence {
                self.record_to_sequence(ctx.midi_in.iter(), ctx.transport);
            }
        }
        if let Some(p) = self.plugin.as_mut() {
            let midi_in = ctx.tmp_midi_buffer.iter().map(|(a, b)| (*a, b));
            p.process_batch(midi_in, &mut self.output);
        }
    }

    fn sequence_to_midi_frames(
        &self,
        dst: &mut Vec<(u32, MidiMessage<'static>)>,
        transport: &Transport,
    ) {
        if self.sequence.is_empty() {
            return;
        }
        let initial_len = dst.len();
        let placeholder_event = MidiEvent {
            position: Position::MAX,
            midi: MidiMessage::Reserved(0),
        };
        let transport_start = transport.iter_transport().next().unwrap_or_default();
        // TODO: Use binary search for performance improvement.
        let start = self
            .sequence
            .iter()
            .position(|e| e.position >= transport_start.start)
            .unwrap_or(self.sequence.len());
        let mut sequence_iter = self
            .sequence
            .iter()
            .chain(std::iter::once(&placeholder_event))
            .cycle()
            .skip(start)
            .peekable();
        for (frame, rng) in transport.iter_transport().enumerate() {
            let is_in_range = |event: &&MidiEvent| {
                if rng.start <= rng.end {
                    rng.contains(&event.position)
                } else {
                    !(rng.end..rng.start).contains(&event.position)
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
        // Sorting is not required under the following:
        //   - The sequence initial length was 0. New items should be inserted in sorted manner.
        //   - The final length did not change. No new items were inserted.
        let is_sorted = initial_len == 0 || dst.len() == initial_len;
        if !is_sorted {
            dst.sort_by_key(|(frame, _)| *frame)
        }
    }

    fn record_to_sequence<'a>(
        &mut self,
        midi_iter: impl 'a + Iterator<Item = &'a (u32, MidiMessage<'static>)>,
        transport: &Transport,
    ) {
        let mut did_change = false;
        for (frame, midi) in midi_iter {
            let position = transport.range_for_frame(*frame).start;
            self.sequence.push(MidiEvent {
                position,
                midi: midi.clone(),
            });
            did_change = true;
        }
        if did_change {
            self.sequence.sort_by_key(|e| e.position);
        }
    }
}

#[cfg(test)]
mod tests {
    use bats_dsp::{position::Position, sample_rate::SampleRate};
    use wmidi::{Channel, Note, U7};

    use super::*;

    const NOTE_ON: MidiMessage<'static> = MidiMessage::NoteOn(Channel::Ch1, Note::C3, U7::MAX);
    const NOTE_OFF: MidiMessage<'static> = MidiMessage::NoteOff(Channel::Ch1, Note::C3, U7::MIN);

    #[test]
    fn empty_sequence_no_midi_produces_silence() {
        let sample_rate = SampleRate::new(44100.0);
        let buffer_size = 256;
        let mut track = Track {
            plugin: Some(Toof::new(sample_rate)),
            volume: 1.0,
            output: Buffers::new(buffer_size),
            sequence: Vec::new(),
        };
        assert!(track.output.is_zero());
        let mut midi = Vec::new();
        track.process(TrackProcessContext {
            record_to_sequence: false,
            transport: &Transport::new_prepopulated(sample_rate, buffer_size, 120.0),
            midi_in: &[],
            tmp_midi_buffer: &mut midi,
        });
        assert!(track.output.is_zero());
        assert_eq!(midi, vec![]);
    }

    #[test]
    fn sequence_produces_sound() {
        let sample_rate = SampleRate::new(44100.0);
        let buffer_size = 256;
        let mut track = Track {
            plugin: Some(Toof::new(sample_rate)),
            volume: 1.0,
            output: Buffers::new(buffer_size),
            sequence: vec![MidiEvent {
                position: Position::MIN,
                midi: NOTE_ON,
            }],
        };
        assert!(track.output.is_zero());
        let mut midi = Vec::new();
        track.process(TrackProcessContext {
            record_to_sequence: false,
            transport: &Transport::new_prepopulated(sample_rate, buffer_size, 120.0),
            midi_in: &[],
            tmp_midi_buffer: &mut midi,
        });
        assert!(!track.output.is_zero());
        assert_eq!(midi, vec![(0, NOTE_ON)]);
    }

    #[test]
    fn sequence_out_of_range_of_transport_remains_silent() {
        let sample_rate = SampleRate::new(44100.0);
        let buffer_size = 256;
        let mut track = Track {
            plugin: Some(Toof::new(sample_rate)),
            volume: 1.0,
            output: Buffers::new(buffer_size),
            sequence: vec![MidiEvent {
                position: Position::new(1000.0),
                midi: NOTE_ON,
            }],
        };
        assert!(track.output.is_zero());
        let mut midi = Vec::new();
        track.process(TrackProcessContext {
            record_to_sequence: false,
            transport: &Transport::new_prepopulated(sample_rate, buffer_size, 120.0),
            midi_in: &[],
            tmp_midi_buffer: &mut midi,
        });
        assert!(track.output.is_zero());
        assert_eq!(midi, vec![]);
    }

    #[test]
    fn midi_produces_sound() {
        let sample_rate = SampleRate::new(44100.0);
        let buffer_size = 256;
        let mut track = Track {
            plugin: Some(Toof::new(sample_rate)),
            volume: 1.0,
            output: Buffers::new(buffer_size),
            sequence: Vec::new(),
        };
        assert!(track.output.is_zero());
        let mut midi = Vec::new();
        track.process(TrackProcessContext {
            record_to_sequence: false,
            transport: &Transport::new_prepopulated(sample_rate, buffer_size, 120.0),
            midi_in: &[(0, NOTE_ON)],
            tmp_midi_buffer: &mut midi,
        });
        assert!(!track.output.is_zero());
        assert_eq!(midi, vec![(0, NOTE_ON)]);
    }

    #[test]
    fn midi_and_sequence_and_combined() {
        let sample_rate = SampleRate::new(44100.0);
        let buffer_size = 256;

        let transport = Transport::new_prepopulated(sample_rate, buffer_size, 120.0);
        let sequence = vec![
            MidiEvent {
                position: Position::new(0.0),
                midi: NOTE_ON,
            },
            MidiEvent {
                position: transport.iter_transport().nth(100).unwrap().start,
                midi: NOTE_OFF,
            },
        ];
        let mut track = Track {
            plugin: Some(Toof::new(sample_rate)),
            volume: 1.0,
            output: Buffers::new(buffer_size),
            sequence,
        };
        let mut midi = Vec::new();
        track.process(TrackProcessContext {
            record_to_sequence: false,
            transport: &transport,
            midi_in: &[(10, NOTE_OFF), (20, NOTE_ON)],
            tmp_midi_buffer: &mut midi,
        });
        assert_eq!(
            midi,
            vec![(0, NOTE_ON), (10, NOTE_OFF), (20, NOTE_ON), (100, NOTE_OFF),]
        );
    }

    #[test]
    fn midi_with_no_record_does_not_fill_sequence() {
        let sample_rate = SampleRate::new(44100.0);
        let buffer_size = 256;
        let mut track = Track {
            plugin: Some(Toof::new(sample_rate)),
            volume: 1.0,
            output: Buffers::new(buffer_size),
            sequence: Vec::new(),
        };
        assert!(track.output.is_zero());
        assert!(track.sequence.is_empty());
        track.process(TrackProcessContext {
            record_to_sequence: false,
            transport: &Transport::new_prepopulated(sample_rate, buffer_size, 120.0),
            midi_in: &[(0, NOTE_ON)],
            tmp_midi_buffer: &mut Vec::new(),
        });
        assert!(!track.output.is_zero());
        assert_eq!(track.sequence, vec![]);
    }

    #[test]
    fn midi_with_record_fills_sequence() {
        let sample_rate = SampleRate::new(44100.0);
        let buffer_size = 256;
        let mut track = Track {
            plugin: Some(Toof::new(sample_rate)),
            volume: 1.0,
            output: Buffers::new(buffer_size),
            sequence: Vec::new(),
        };
        assert!(track.output.is_zero());
        assert!(track.sequence.is_empty());
        let transport = Transport::new_prepopulated(sample_rate, buffer_size, 120.0);
        track.process(TrackProcessContext {
            record_to_sequence: true,
            transport: &transport,
            midi_in: &[(40, NOTE_ON)],
            tmp_midi_buffer: &mut Vec::new(),
        });
        assert!(!track.output.is_zero());
        assert_eq!(
            track.sequence,
            vec![MidiEvent {
                position: transport.iter_transport().nth(40).unwrap().start,
                midi: NOTE_ON
            }]
        );
    }
}
