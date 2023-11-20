use serde::{Deserialize, Serialize};

use crate::sample_rate::SampleRate;

/// Position contains the position within the transport. This includes
/// a beat and sub_beat component.
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    /// The beat where the top 32bits represent the beat and the bottom 32 bits represents the sub
    /// beat.
    beat: u64,
}

impl Position {
    /// The minimum position.
    pub const MIN: Position = Position { beat: 0 };

    /// The maximum represntable position.
    pub const MAX: Position = Position { beat: u64::MAX };

    /// The minimum (non-zero) represntable position.
    pub const DELTA: Position = Position { beat: 1 };

    /// Create a new `Position` with the given beat and sub_beat. If
    /// `sub_beat` is greater than 0, then it is converted into the
    /// appropriate amount of sub beats.
    pub fn new(beat: f64) -> Position {
        let sub_beat_scalar = (1u64 << 32) as f64;
        let beat_part = beat.trunc();
        let sub_beat_part = beat.fract() * sub_beat_scalar;
        Position::with_components(beat_part as u32, sub_beat_part as u32)
    }

    /// Create a new `Position` with the given beat and sub beat.
    pub fn with_components(beat: u32, sub_beat: u32) -> Position {
        let higher = (beat as u64) << 32;
        let lower = sub_beat as u64;
        Position {
            beat: higher + lower,
        }
    }

    /// Get the delta for each BPM. This is the amount of position that advances for every sample.
    pub fn delta_from_bpm(sample_rate: SampleRate, bpm: f32) -> Position {
        let beats_per_second = bpm / 60.0;
        Position::new(beats_per_second as f64 * sample_rate.seconds_per_sample() as f64)
    }

    /// Get the beat for `self`.
    pub fn beat(&self) -> u32 {
        (self.beat >> 32) as u32
    }

    /// Get the sub beat for `self`.
    pub fn sub_beat(&self) -> u32 {
        (self.beat & 0x00000000FFFFFFFF) as u32
    }

    /// Set the beat component for `self`.
    pub fn set_beat(&mut self, beat: u32) {
        *self = Position::with_components(beat, self.sub_beat())
    }
}

impl std::ops::Add for Position {
    type Output = Position;

    fn add(self, rhs: Position) -> Position {
        Position {
            beat: self.beat.wrapping_add(rhs.beat),
        }
    }
}

impl std::ops::AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::fmt::Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let whole = self.beat() as f32;
        let fract = self.sub_beat() as f32 / (1u64 << 32) as f32;
        let beat = whole + fract;
        f.debug_struct("Position").field("beat", &beat).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_value_is_zero() {
        assert_eq!(Position::default(), Position::new(0.0));
        assert_eq!(Position::default().beat(), 0);
        assert_eq!(Position::default().sub_beat(), 0);
    }

    #[test]
    fn new_beat_position_with_has_beat_and_sub_beat() {
        let p = Position::new(11.5);
        assert_eq!(p.beat(), 11);
        assert_eq!(p.sub_beat(), ((1u64 << 32) / 2) as u32);
    }

    #[test]
    fn add_beat_adds_components_and_carries_the_sub_beat() {
        assert_eq!(
            Position::new(1.625) + Position::new(3.75),
            Position::new(5.375)
        );
    }

    #[test]
    fn position_wraps_around_on_add() {
        assert_eq!(Position::MAX + Position::DELTA, Position::new(0.0));
        assert_eq!(
            Position::MAX + (Position::DELTA + Position::new(1.0)),
            Position::new(1.0)
        );
    }

    #[test]
    fn position_delta_from_beats() {
        assert_eq!(
            Position::delta_from_bpm(SampleRate::new(16.0), 60.0),
            Position::new(1.0 / 16.0)
        );
    }

    #[test]
    fn to_debug() {
        let debug_string = format!("{:?}", Position::new(4.5));
        assert!(
            debug_string.contains("4.5"),
            "\"{debug_string}\" does not contain 4.5."
        );
    }
}
