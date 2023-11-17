use bats_dsp::buffers::Buffers;
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
        ctx.transport
            .sequence_to_frames(ctx.tmp_midi_buffer, &self.sequence);
        if !ctx.midi_in.is_empty() {
            let should_sort = !ctx.tmp_midi_buffer.is_empty();
            ctx.tmp_midi_buffer.extend_from_slice(ctx.midi_in);
            if should_sort {
                ctx.tmp_midi_buffer.sort_by_key(|(frame, _)| *frame);
            }
            if ctx.record_to_sequence {
                ctx.transport
                    .push_to_sequence(&mut self.sequence, ctx.midi_in.iter());
            }
        }
        if let Some(p) = self.plugin.as_mut() {
            let midi_in = ctx.tmp_midi_buffer.iter().map(|(a, b)| (*a, b));
            p.process_batch(midi_in, &mut self.output);
        }
    }
}
