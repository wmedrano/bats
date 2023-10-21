pub mod toof;

/// Defines a generic instrument plugin.
pub trait BatsInstrument {
    /// Create a new plugin.
    fn new(sample_rate: f32) -> Self;

    /// The name of the plugin.
    fn name(&self) -> &'static str;

    /// Handle processing of `midi_in` and output to `left_out` and
    /// `right_out`.
    fn process(
        &mut self,
        midi_in: &[(u32, wmidi::MidiMessage<'static>)],
        left_out: &mut [f32],
        right_out: &mut [f32],
    );

    /// Handle processing of `midi_in` and return the results. This is
    /// often less efficient but is included for less performance
    /// critical use cases like unit tests.
    fn process_to_vec(
        &mut self,
        sample_count: usize,
        midi_in: &[(u32, wmidi::MidiMessage<'static>)],
    ) -> (Vec<f32>, Vec<f32>) {
        let (mut left, mut right) = (vec![0f32; sample_count], vec![0f32; sample_count]);
        self.process(midi_in, &mut left, &mut right);
        (left, right)
    }
}
