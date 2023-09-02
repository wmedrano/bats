//! The main processing logic for bats!.
use crate::plugins::plugin_trait::GenericPlugin;
use crate::plugins::Plugin;

/// Handles the main audio processing for bats!.
#[derive(Debug)]
pub struct Processor {
    /// The volume output. Essentially a multiplier for the signal.
    pub volume: f32,

    /// The set of plugins.
    pub plugins: Vec<Plugin>,

    /// Contains enough audio data for two channels.
    tmp_duplex_buffer: Vec<f32>,
}

impl Processor {
    /// Create a new `Processor` that can support buffer sizes of up to `max_buffer_size`.
    pub fn new(max_buffer_size: usize) -> Processor {
        Processor {
            volume: 1.0,
            plugins: Vec::with_capacity(64),
            tmp_duplex_buffer: vec![0.0; max_buffer_size * 2],
        }
    }

    /// Perform processing and return the audio data to a vector.
    ///
    /// Note: Prefer pre-allocating audio buffers and using `process`
    /// for improved performance.
    #[cfg(test)]
    pub fn process_to_vec<'a>(
        &mut self,
        frames: usize,
        midi_in: impl Clone + Iterator<Item = (u32, &'a [u8])>,
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
        midi_in: impl Clone + Iterator<Item = (u32, &'a [u8])>,
        out_left: &mut [f32],
        out_right: &mut [f32],
    ) {
        clear_buffer(out_left);
        clear_buffer(out_right);
        assert!(out_left.len() + out_right.len() <= self.tmp_duplex_buffer.len());
        let (buffer_a, buffer_b) = self.tmp_duplex_buffer.split_at_mut(out_left.len());
        for plugin in self.plugins.iter_mut() {
            plugin.process(midi_in.clone(), buffer_a, buffer_b);
            mix_to_buffer(out_left, buffer_a);
            mix_to_buffer(out_right, buffer_b);
        }
        for buffer in [out_left, out_right] {
            for v in buffer.iter_mut() {
                *v *= self.volume;
            }
        }
    }
}

fn clear_buffer(b: &mut [f32]) {
    for v in b.iter_mut() {
        *v = 0.0;
    }
}

fn mix_to_buffer(dst: &mut [f32], src: &[f32]) {
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        *d += *s;
    }
}

#[cfg(test)]
mod tests {
    use crate::{plugins::sampler::OneShotSampler, sample::Sample};

    use super::*;

    #[should_panic]
    #[test]
    fn test_processor_process_with_large_input_causes_panic() {
        let mut processor = Processor::new(8);
        processor.process_to_vec(16, std::iter::empty());
    }

    #[test]
    fn test_processor_inaction_produces_silence() {
        let mut processor = Processor::new(8);
        processor
            .plugins
            .push(OneShotSampler::new(Sample::with_mono_data(&[1.0])).into());
        let (left, right) = processor.process_to_vec(2, std::iter::empty());
        assert_eq!(left, vec![0.0, 0.0]);
        assert_eq!(right, vec![0.0, 0.0]);
    }

    #[test]
    fn test_processor_pressing_note_produces_sound_on_same_frame() {
        let mut processor = Processor::new(8);
        processor
            .plugins
            .push(OneShotSampler::new(Sample::with_mono_data(&[1.0])).into());
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
        let mut processor = Processor::new(8);
        processor
            .plugins
            .push(OneShotSampler::new(Sample::with_mono_data(&[1.0])).into());
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
        let mut processor = Processor::new(8);
        processor
            .plugins
            .push(OneShotSampler::new(Sample::with_mono_data(&[1.0, 0.5])).into());
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

    #[test]
    fn test_processor_add_multiple_plugins_sums_signals() {
        let mut processor = Processor::new(8);
        processor
            .plugins
            .push(OneShotSampler::new(Sample::with_mono_data(&[0.1, 0.2])).into());
        processor
            .plugins
            .push(OneShotSampler::new(Sample::with_mono_data(&[0.01, 0.02])).into());
        let note_on = wmidi::MidiMessage::NoteOn(
            wmidi::Channel::Ch1,
            wmidi::Note::C1,
            wmidi::U7::from_u8_lossy(100),
        )
        .to_vec();

        let (left, right) = processor.process_to_vec(2, std::iter::once((0, note_on.as_slice())));
        assert_eq!(left, vec![0.11, 0.22]);
        assert_eq!(right, vec![0.11, 0.22]);
    }
}
