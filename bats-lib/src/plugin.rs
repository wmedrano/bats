use anyhow::anyhow;
use bats_dsp::buffers::Buffers;

use self::metadata::Metadata;

pub mod metadata;
pub mod toof;

/// Defines a generic instrument plugin.
pub trait BatsInstrument {
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

    /// Run any batch cleanup operations.
    fn batch_cleanup(&mut self) {}

    /// Handle processing of `midi_in` and output to `left_out` and
    /// `right_out`.
    ///
    /// Prefer using this default behavior unless benchmarking shows significant performance
    /// improvements.
    fn process_batch(
        &mut self,
        midi_in: &[(u32, wmidi::MidiMessage<'static>)],
        output: &mut Buffers,
    ) {
        let sample_count = output.len();
        let mut midi_iter = midi_in.iter().peekable();
        for i in 0..sample_count {
            while let Some((_, msg)) = midi_iter.next_if(|(frame, _)| *frame <= i as u32) {
                self.handle_midi(msg);
            }
            output.set(i, self.process())
        }
        self.batch_cleanup();
    }
}

pub trait BatsInstrumentExt: BatsInstrument {
    /// Handle processing of `midi_in` and return the results. This is
    /// often less efficient but is included for less performance
    /// critical use cases like unit tests.
    fn process_to_buffers(
        &mut self,
        sample_count: usize,
        midi_in: &[(u32, wmidi::MidiMessage<'static>)],
    ) -> Buffers {
        let mut buffers = Buffers::new(sample_count);
        self.process_batch(midi_in, &mut buffers);
        buffers
    }

    /// Set a parameter value.
    fn set_param_by_name(&mut self, name: &'static str, value: f32) -> anyhow::Result<()> {
        let metadata = self.metadata();
        let param = match metadata.param_by_name(name) {
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

#[cfg(test)]
mod tests {
    use bats_dsp::sample_rate::SampleRate;

    use super::{toof::Toof, *};

    #[test]
    fn set_param_by_name_sets_param_by_name() {
        let mut manual = Toof::new(SampleRate::new(44100.0));
        let mut by_name = Toof::new(SampleRate::new(44100.0));

        manual.set_param(2, 432.0);
        assert_ne!(manual, by_name);
        by_name.set_param_by_name("filter cutoff", 432.0).unwrap();
        assert_eq!(manual, by_name);
    }

    #[test]
    fn set_param_by_name_with_bad_name_returns_error() {
        let mut plugin = Toof::new(SampleRate::new(44100.0));
        let param_name = "Name that does not exist.";
        assert!(plugin.set_param_by_name(param_name, 0.0).is_err());
    }
}
