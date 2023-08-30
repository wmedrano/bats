//! Sampler plugin integrations.
use wmidi::MidiMessage;

use crate::sample::{Sample, SampleIter};

use super::plugin_trait::GenericPlugin;

/// A sampler that plays a sample on each note on and cuts off on each
/// note off.
#[derive(Clone, Debug)]
pub struct OneShotSampler {
    /// An iterator over the sample.
    sample_iter: SampleIter,
}

impl OneShotSampler {
    /// Create a new `OneShotSampler` with `sample`.
    pub fn new(sample: Sample) -> OneShotSampler {
        let mut sample_iter = sample.iter_samples();
        sample_iter.end();
        OneShotSampler { sample_iter }
    }
}

impl GenericPlugin for OneShotSampler {
    /// Process a single chunk of audio.
    fn process<'a>(
        &mut self,
        midi_in: impl Iterator<Item = (u32, &'a [u8])>,
        left_out: &mut [f32],
        right_out: &mut [f32],
    ) {
        let mut midi_in = midi_in.peekable();
        let frames = left_out.len().min(right_out.len());
        for frame in 0..frames {
            while let Some((_, msg_bytes)) = midi_in.next_if(|(f, _)| *f <= frame as u32) {
                match MidiMessage::from_bytes(msg_bytes) {
                    Ok(MidiMessage::NoteOn(_, _, _)) => self.sample_iter.reset(),
                    Ok(MidiMessage::NoteOff(_, _, _)) | Ok(MidiMessage::Reset) => {
                        self.sample_iter.end()
                    }
                    _ => (),
                }
            }
            (left_out[frame], right_out[frame]) = self.sample_iter.next().unwrap_or((0.0, 0.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use wmidi::MidiMessage;

    use super::*;

    #[test]
    fn test_sampler_mismatch_channel_lengths_processes_the_min_length() {
        let mut sampler = OneShotSampler::new(Sample::with_mono_data(&[1.0, 1.0, 1.0]));
        let mut left = [0.0];
        let mut right = [0.0, 0.0];
        let note_on = MidiMessage::NoteOn(
            wmidi::Channel::Ch1,
            wmidi::Note::C4,
            wmidi::U7::from_u8_lossy(100),
        )
        .to_vec();
        sampler.process(
            std::iter::once((0, note_on.as_slice())),
            &mut left,
            &mut right,
        );

        // First frame for both.
        assert_ne!(left, [0.0]);
        assert_ne!(right[0], 0.0);
        // Second frame.
        assert_eq!(right[1], 0.0);
    }

    #[test]
    fn test_lv2_process_with_no_press_is_silent() {
        let mut sampler = OneShotSampler::new(Sample::with_mono_data(&[1.0, 1.0, 1.0]));
        let (left, right) = sampler.process_to_vec(2, std::iter::empty());
        assert_eq!(left, vec![0.0, 0.0]);
        assert_eq!(right, vec![0.0, 0.0]);
    }

    #[test]
    fn test_lv2_process_with_note_on_produces_audio() {
        let mut sampler = OneShotSampler::new(Sample::with_mono_data(&[1.0, 1.0, 1.0]));
        let note_on = MidiMessage::NoteOn(
            wmidi::Channel::Ch1,
            wmidi::Note::C4,
            wmidi::U7::from_u8_lossy(100),
        )
        .to_vec();
        let (left, right) = sampler.process_to_vec(2, std::iter::once((0, note_on.as_slice())));
        assert_ne!(left, vec![0.0, 0.0]);
        assert_ne!(right, vec![0.0, 0.0]);
    }
}
