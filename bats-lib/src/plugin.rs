use anyhow::anyhow;
use bats_dsp::{buffers::Buffers, sample_rate::SampleRate};

use self::metadata::Metadata;

pub mod metadata;
pub mod toof;

/// Defines a generic instrument plugin.
pub trait BatsInstrument {
    /// Create a new plugin.
    fn new(sample_rate: SampleRate) -> Self;

    /// The name of the plugin.
    fn metadata(&self) -> &'static Metadata;

    /// Handle a midi message.
    fn handle_midi(&mut self, msg: &wmidi::MidiMessage);

    /// Produce the next samples in the frame.
    fn process(&mut self) -> (f32, f32);

    /// Get the value of the parameter.
    fn param(&self, id: u32) -> f32;

    /// Set a parameter.
    fn set_param(&mut self, id: u32, value: f32);

    /// Handle processing of `midi_in` and output to `left_out` and
    /// `right_out`.
    fn process_batch(
        &mut self,
        midi_in: &[(u32, wmidi::MidiMessage<'static>)],
        left_out: &mut [f32],
        right_out: &mut [f32],
    ) {
        let sample_count = left_out.len().min(right_out.len());
        let mut midi_iter = midi_in.iter().peekable();
        for i in 0..sample_count {
            while let Some((_, msg)) = midi_iter.next_if(|(frame, _)| *frame <= i as u32) {
                self.handle_midi(msg);
            }
            let (l, r) = self.process();
            left_out[i] = l;
            right_out[i] = r;
        }
    }
}

pub trait BatsInstrumentExt: BatsInstrument {
    /// Handle processing of `midi_in` and return the results. This is
    /// often less efficient but is included for less performance
    /// critical use cases like unit tests.
    #[cold]
    fn process_to_buffers(
        &mut self,
        sample_count: usize,
        midi_in: &[(u32, wmidi::MidiMessage<'static>)],
    ) -> Buffers {
        let mut buffers = Buffers::new(sample_count);
        self.process_batch(midi_in, &mut buffers.left, &mut buffers.right);
        buffers
    }

    /// Set a parameter value.
    #[cold]
    fn set_param_by_name(&mut self, name: &'static str, value: f32) -> anyhow::Result<()> {
        let metadata = self.metadata();
        let param = match metadata.params.iter().find(|p| p.name == name) {
            None => {
                return Err(anyhow!(
                    "Plugin {} not found. Valid values are: {:?}",
                    name,
                    metadata.params.iter().map(|p| p.name)
                ))
            }
            Some(p) => p,
        };
        self.set_param(param.id, value);
        Ok(())
    }
}

impl<T: BatsInstrument> BatsInstrumentExt for T {}
