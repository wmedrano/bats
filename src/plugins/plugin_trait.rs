//! Contains the trait for plugins.

/// The trait that all plugins must implement.
pub trait GenericPlugin {
    /// Process a single chunk of audio.
    fn process<'a>(
        &mut self,
        midi_in: impl Iterator<Item = (u32, &'a [u8])>,
        left_out: &mut [f32],
        right_out: &mut [f32],
    );

    /// Process a single chunk of audio and output it to a vector.
    ///
    /// Prefer using `process` for performance critical uses.
    fn process_to_vec<'a>(
        &mut self,
        samples: usize,
        midi_in: impl Iterator<Item = (u32, &'a [u8])>,
    ) -> (Vec<f32>, Vec<f32>) {
        let mut left = vec![0.0; samples];
        let mut right = vec![0.0; samples];
        self.process(midi_in, &mut left, &mut right);
        (left, right)
    }
}
