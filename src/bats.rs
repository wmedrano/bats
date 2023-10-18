#[derive(Default, Debug, Clone)]
pub struct Bats {}

impl Bats {
    /// Process midi data and output audio.
    pub fn process<'a>(
        &mut self,
        midi: impl Clone + Iterator<Item = &'a (u32, wmidi::MidiMessage<'static>)>,
        left: &mut [f32],
        right: &mut [f32],
    ) {
        for _ in midi {}
        left.fill(0.0);
        right.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_input_produces_empty_output() {
        let mut left = [1.0, 2.0, 3.0];
        let mut right = [4.0, 5.0, 6.0];
        Bats::default().process(std::iter::empty(), &mut left, &mut right);
        assert_eq!(left, [0.0, 0.0, 0.0]);
        assert_eq!(right, [0.0, 0.0, 0.0]);
    }
}
