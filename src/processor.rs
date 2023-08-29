//! The main processing logic for bats!.
use crate::sample::{Sample, SampleIter};

#[derive(Debug)]
pub struct Processor {
    tick_sample: SampleIter,
}

impl Default for Processor {
    fn default() -> Processor {
        let mut tick_sample = Sample::with_stereo_data(&[1.0, -1.0], &[1.0, -1.0])
            .unwrap()
            .iter_samples();
        while tick_sample.next().is_some() {}
        Processor { tick_sample }
    }
}

impl Processor {
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
            (*left.next().unwrap(), *right.next().unwrap()) =
                self.tick_sample.next().unwrap_or((0.0, 0.0));
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
        let mut left = vec![1.0, 1.0];
        let mut right = vec![-1.0, -1.0];
        processor.process(std::iter::empty(), &mut left, &mut right);
        assert_eq!(left, vec![0.0, 0.0]);
        assert_eq!(right, vec![0.0, 0.0]);
    }

    #[test]
    fn test_processor_pressing_note_produces_sound_on_same_frame() {
        let mut processor = Processor::default();
        let note_on = midi_to_vec(wmidi::MidiMessage::NoteOn(
            wmidi::Channel::Ch1,
            wmidi::Note::C1,
            wmidi::U7::from_u8_lossy(100),
        ));
        let mut left = vec![0.0, 0.0];
        let mut right = vec![0.0, 0.0];
        processor.process(
            std::iter::once((1, note_on.as_slice())),
            &mut left,
            &mut right,
        );
        assert_eq!(left, vec![0.0, 1.0]);
        assert_eq!(right, vec![0.0, 1.0]);
    }

    #[test]
    fn test_processor_releasing_note_cuts_off_sound() {
        let mut processor = Processor::default();
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
        let mut left = vec![0.0, 0.0];
        let mut right = vec![0.0, 0.0];
        processor.process(
            [(1, note_on.as_slice()), (1, note_off.as_slice())].into_iter(),
            &mut left,
            &mut right,
        );
        assert_eq!(left, vec![0.0, 0.0]);
        assert_eq!(right, vec![0.0, 0.0]);
    }
}
