//! The main processing logic for bats!.
use crate::plugins::Plugin;
use crate::{
    plugins::plugin_trait::GenericPlugin, plugins::sampler::OneShotSampler, sample::Sample,
};

/// Handles the main audio processing for bats!.
#[derive(Debug)]
pub struct Processor {
    /// The volume output. Essentially a multiplier for the signal.
    pub volume: f32,

    /// The current active plugin.
    pub plugin: Plugin,
}

impl Default for Processor {
    /// Create a default version of `Processor`.
    fn default() -> Processor {
        let sample = Sample::with_mono_data(&[1.0, -1.0]);
        let sampler = OneShotSampler::new(sample);
        Processor {
            volume: 1.0,
            plugin: Plugin::from(sampler),
        }
    }
}

impl Processor {
    /// Perform processing and return the audio data to a vector.
    ///
    /// Note: Prefer pre-allocating audio buffers and using `process`
    /// for improved performance.
    #[cfg(test)]
    pub fn process_to_vec<'a>(
        &mut self,
        frames: usize,
        midi_in: impl Iterator<Item = (u32, &'a [u8])>,
    ) -> (Vec<f32>, Vec<f32>)
where {
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];
        self.process(midi_in, &mut left, &mut right);
        (left, right)
    }

    /// Perform processing for a single buffer.
    pub fn process<'a>(
        &mut self,
        midi_in: impl Iterator<Item = (u32, &'a [u8])>,
        out_left: &mut [f32],
        out_right: &mut [f32],
    ) {
        self.plugin.process(midi_in, out_left, out_right);
        for buffer in [out_left, out_right] {
            for v in buffer.iter_mut() {
                *v *= self.volume;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_inaction_produces_silence() {
        let mut processor = Processor::default();
        processor.plugin = OneShotSampler::new(Sample::with_mono_data(&[1.0])).into();
        let (left, right) = processor.process_to_vec(2, std::iter::empty());
        assert_eq!(left, vec![0.0, 0.0]);
        assert_eq!(right, vec![0.0, 0.0]);
    }

    #[test]
    fn test_processor_pressing_note_produces_sound_on_same_frame() {
        let mut processor = Processor::default();
        processor.plugin = OneShotSampler::new(Sample::with_mono_data(&[1.0])).into();
        let note_on = wmidi::MidiMessage::NoteOn(
            wmidi::Channel::Ch1,
            wmidi::Note::C1,
            wmidi::U7::from_u8_lossy(100),
        )
        .to_vec();
        let (left, right) = processor.process_to_vec(2, std::iter::once((1, note_on.as_slice())));
        assert_eq!(left, vec![0.0, 1.0]);
        assert_eq!(right, vec![0.0, 1.0]);
    }

    #[test]
    fn test_processor_releasing_note_cuts_off_sound() {
        let mut processor = Processor::default();
        processor.plugin = OneShotSampler::new(Sample::with_mono_data(&[1.0])).into();
        let note_on = wmidi::MidiMessage::NoteOn(
            wmidi::Channel::Ch1,
            wmidi::Note::C1,
            wmidi::U7::from_u8_lossy(100),
        )
        .to_vec();
        let note_off = wmidi::MidiMessage::NoteOff(
            wmidi::Channel::Ch1,
            wmidi::Note::C1,
            wmidi::U7::from_u8_lossy(100),
        )
        .to_vec();
        let (left, right) = processor.process_to_vec(
            2,
            [(1, note_on.as_slice()), (1, note_off.as_slice())].into_iter(),
        );
        assert_eq!(left, vec![0.0, 0.0]);
        assert_eq!(right, vec![0.0, 0.0]);
    }

    #[test]
    fn test_changing_output_volume_scales_final_output() {
        let mut processor = Processor::default();
        processor.plugin = OneShotSampler::new(Sample::with_mono_data(&[1.0, 0.5])).into();
        let note_on = wmidi::MidiMessage::NoteOn(
            wmidi::Channel::Ch1,
            wmidi::Note::C1,
            wmidi::U7::from_u8_lossy(100),
        )
        .to_vec();

        processor.volume = 0.2;
        let (left, right) = processor.process_to_vec(2, std::iter::once((0, note_on.as_slice())));
        assert_eq!(left, vec![0.2, 0.1]);
        assert_eq!(right, vec![0.2, 0.1]);
    }
}
