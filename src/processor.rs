//! The main processing logic for bats!.
use crate::sample::{Sample, SampleIter};

#[derive(Debug)]
pub struct Processor {
    /// The volume output. Essentially a multiplier for the signal.
    pub volume: f32,

    /// An iterator over the current sample.
    tick_sample: SampleIter,
}

impl Default for Processor {
    /// Create a default version of `Processor`.
    fn default() -> Processor {
        let tick_sample = Sample::with_mono_data(&[1.0, -1.0]).iter_samples();
        Processor {
            volume: 1.0,
            tick_sample,
        }
    }
}

impl Processor {
    /// Set the sample to `sample`.
    #[cfg(test)]
    pub fn set_sample(&mut self, sample: Sample) {
        self.tick_sample = sample.iter_samples();
        self.tick_sample.end();
    }

    /// Perform processing and return the audio data to a vector.
    ///
    /// Note: Prefer pre-allocating audio buffers and using `process`
    /// for improved performance.
    pub fn process_to_vec<'a>(
        &mut self,
        frames: usize,
        midi_in: impl Iterator<Item = (u32, &'a [u8])>,
    ) -> (Vec<f32>, Vec<f32>) {
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
        let frames = out_left.len();
        let mut left = out_left.iter_mut();
        let mut right = out_right.iter_mut();
        let mut midi = midi_in.peekable();
        for frame_idx in 0..frames {
            while let Some((_, msg)) = midi.next_if(|(f, _)| *f <= frame_idx as u32) {
                match wmidi::MidiMessage::try_from(msg) {
                    Ok(wmidi::MidiMessage::NoteOn(_, _, _)) => {
                        self.tick_sample.reset();
                    }
                    Ok(wmidi::MidiMessage::NoteOff(_, _, _)) => {
                        self.tick_sample.end();
                    }
                    _ => {}
                }
            }
            let (a, b) = self.tick_sample.next().unwrap_or((0.0, 0.0));
            (*left.next().unwrap(), *right.next().unwrap()) = (a * self.volume, b * self.volume)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn midi_to_vec(msg: wmidi::MidiMessage) -> Vec<u8> {
        let size = msg.bytes_size();
        let mut ret = vec![0u8; size];
        msg.copy_to_slice(&mut ret).unwrap();
        ret
    }

    #[test]
    fn test_processor_inaction_produces_silence() {
        let mut processor = Processor::default();
        processor.set_sample(Sample::with_mono_data(&[1.0]));
        let (left, right) = processor.process_to_vec(2, std::iter::empty());
        assert_eq!(left, vec![0.0, 0.0]);
        assert_eq!(right, vec![0.0, 0.0]);
    }

    #[test]
    fn test_processor_pressing_note_produces_sound_on_same_frame() {
        let mut processor = Processor::default();
        processor.set_sample(Sample::with_mono_data(&[1.0]));
        let note_on = midi_to_vec(wmidi::MidiMessage::NoteOn(
            wmidi::Channel::Ch1,
            wmidi::Note::C1,
            wmidi::U7::from_u8_lossy(100),
        ));
        let (left, right) = processor.process_to_vec(2, std::iter::once((1, note_on.as_slice())));
        assert_eq!(left, vec![0.0, 1.0]);
        assert_eq!(right, vec![0.0, 1.0]);
    }

    #[test]
    fn test_processor_releasing_note_cuts_off_sound() {
        let mut processor = Processor::default();
        processor.set_sample(Sample::with_mono_data(&[1.0]));
        let note_on = midi_to_vec(wmidi::MidiMessage::NoteOn(
            wmidi::Channel::Ch1,
            wmidi::Note::C1,
            wmidi::U7::from_u8_lossy(100),
        ));
        let note_off = midi_to_vec(wmidi::MidiMessage::NoteOff(
            wmidi::Channel::Ch1,
            wmidi::Note::C1,
            wmidi::U7::from_u8_lossy(100),
        ));
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
        processor.set_sample(Sample::with_mono_data(&[1.0, 0.5]));
        let note_on = midi_to_vec(wmidi::MidiMessage::NoteOn(
            wmidi::Channel::Ch1,
            wmidi::Note::C1,
            wmidi::U7::from_u8_lossy(100),
        ));

        processor.volume = 0.2;
        let (left, right) = processor.process_to_vec(2, std::iter::once((0, note_on.as_slice())));
        assert_eq!(left, vec![0.2, 0.1]);
        assert_eq!(right, vec![0.2, 0.1]);
    }
}
