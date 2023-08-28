#[derive(Default, Debug)]
pub struct Processor {}

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
            let l = left.next().unwrap();
            let r = right.next().unwrap();
            *l = 0.0;
            *r = 0.0;
            while let Some((_, msg)) = midi.next_if(|(f, _)| *f <= frame_idx as u32) {
                match wmidi::MidiMessage::try_from(msg) {
                    Ok(wmidi::MidiMessage::NoteOn(_, _, _)) => {
                        *l = 1.0;
                        *r = 1.0;
                    }
                    Ok(wmidi::MidiMessage::NoteOff(_, _, _)) => {
                        *l = -1.0;
                        *r = -1.0;
                    }
                    _ => {}
                }
            }
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
        let (frame, msg) = (
            1,
            midi_to_vec(wmidi::MidiMessage::NoteOn(
                wmidi::Channel::Ch1,
                wmidi::Note::C1,
                wmidi::U7::from_u8_lossy(100),
            )),
        );
        let mut left = vec![0.0, 0.0, 0.0];
        let mut right = vec![0.0, 0.0, 0.0];
        processor.process(
            std::iter::once((frame, msg.as_slice())),
            &mut left,
            &mut right,
        );
        assert_eq!(left, vec![0.0, 1.0, 0.0]);
        assert_eq!(right, vec![0.0, 1.0, 0.0]);
    }
}
