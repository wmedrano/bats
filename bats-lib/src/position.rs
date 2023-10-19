/// Position contains the position within the transport. This includes
/// a beat and sub_beat component.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Position {
    beat: u64,
    sub_beat: f64,
}

impl Position {
    /// Create a new `Position` with the given beat and sub_beat. If
    /// `sub_beat` is greater than 0, than it is converted into the
    /// appropriate amount of beats.
    pub fn new(beat: u64, sub_beat: f64) -> Position {
        if sub_beat < 1.0 {
            Position { beat, sub_beat }
        } else {
            Position {
                beat: beat + sub_beat as u64,
                sub_beat: sub_beat - (sub_beat as u64 as f64),
            }
        }
    }

    /// Get the beat for `self`.
    pub fn beat(&self) -> u64 {
        self.beat
    }

    /// Get the sub beat for `self`.
    pub fn sub_beat(&self) -> f64 {
        self.sub_beat
    }
}

impl std::ops::Add for Position {
    type Output = Position;

    fn add(self, rhs: Position) -> Position {
        let mut ret = self;
        ret += rhs;
        ret
    }
}

impl std::ops::AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        self.sub_beat += rhs.sub_beat;
        if self.sub_beat >= 1.0 {
            self.sub_beat -= 1.0;
            self.beat += 1;
        }
        self.beat += rhs.beat;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_value_is_zero() {
        assert_eq!(Position::default(), Position::new(0, 0.0));
    }

    #[test]
    fn new_beat_position_with_large_sub_beat_is_normalized() {
        assert_eq!(Position::new(1, 10.0), Position::new(11, 0.0));
    }

    #[test]
    fn add_beat_adds_components_and_carries_the_sub_beat() {
        assert_eq!(
            Position::new(1, 0.625) + Position::new(3, 0.75),
            Position::new(5, 0.375)
        );
    }
}
