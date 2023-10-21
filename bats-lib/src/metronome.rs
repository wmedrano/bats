use crate::position::Position;

/// Tracks position according to the specified BPM.
#[derive(Clone, Debug)]
pub struct Metronome {
    position: Position,
    position_per_sample: Position,
}

impl Metronome {
    /// Create a new metronome with the given sample rate and beats per minute.
    pub fn new(sample_rate: f32, bpm: f32) -> Metronome {
        let mut m = Metronome {
            position: Position::default(),
            position_per_sample: Position::default(),
        };
        m.set_bpm(sample_rate, bpm);
        m
    }

    /// Set the beats per minute for a metronome.
    pub fn set_bpm(&mut self, sample_rate: f32, bpm: f32) {
        let seconds_per_sample = 1.0 / sample_rate as f64;
        let beats_per_second = bpm / 60.0;
        self.position_per_sample = Position::new(0, beats_per_second as f64 * seconds_per_sample);
    }

    /// Get the next position from the metronome.
    pub fn next_position(&mut self) -> Position {
        let ret = self.position;
        self.position += self.position_per_sample;
        ret
    }
}

impl Iterator for Metronome {
    type Item = Position;

    fn next(&mut self) -> Option<Position> {
        Some(self.next_position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metronome_produces_beat_at_proper_time() {
        let bpm = 4.0 * 60.0; // 4 beats per second.
        let m = Metronome::new(16.0, bpm);
        assert_eq!(
            m.take(9).collect::<Vec<Position>>(),
            vec![
                Position::new(0, 0.0),
                Position::new(0, 0.25),
                Position::new(0, 0.5),
                Position::new(0, 0.75),
                Position::new(1, 0.0),
                Position::new(1, 0.25),
                Position::new(1, 0.5),
                Position::new(1, 0.75),
                Position::new(2, 0.0),
            ]
        );
    }
}