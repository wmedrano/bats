use bats_dsp::sample_rate::SampleRate;

use crate::position::Position;

/// Tracks position according to the specified BPM.
#[derive(Clone, Debug)]
pub struct Metronome {
    bpm: f32,
    position: Position,
    position_per_sample: Position,
}

impl Metronome {
    /// Create a new metronome with the given sample rate and beats per minute.
    pub fn new(sample_rate: SampleRate, bpm: f32) -> Metronome {
        let mut m = Metronome {
            bpm,
            position: Position::default(),
            position_per_sample: Position::default(),
        };
        m.set_bpm(sample_rate, bpm);
        m
    }

    /// Set the beats per minute for a metronome.
    pub fn set_bpm(&mut self, sample_rate: SampleRate, bpm: f32) {
        self.bpm = bpm;
        let beats_per_second = bpm / 60.0;
        self.position_per_sample =
            Position::new(beats_per_second as f64 * sample_rate.seconds_per_sample() as f64);
    }

    /// Get the next position from the metronome.
    pub fn next_position(&mut self) -> Position {
        let ret = self.position;
        self.position += self.position_per_sample;
        ret
    }

    /// Get the current bpm.
    pub fn bpm(&self) -> f32 {
        self.bpm
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
        let m = Metronome::new(SampleRate::new(16.0), bpm);
        assert_eq!(
            m.take(9).collect::<Vec<Position>>(),
            vec![
                Position::new(0.0),
                Position::new(0.25),
                Position::new(0.5),
                Position::new(0.75),
                Position::new(1.0),
                Position::new(1.25),
                Position::new(1.5),
                Position::new(1.75),
                Position::new(2.0),
            ]
        );
    }
}
