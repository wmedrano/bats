/// Position contains the position within the transport. This includes
/// a beat and sub_beat component.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Position {
    beat: f64,
}

impl Position {
    /// Create a new `Position` with the given beat and sub_beat. If
    /// `sub_beat` is greater than 0, than it is converted into the
    /// appropriate amount of beats.
    pub fn new(beat: f64) -> Position {
        Position { beat }
    }

    /// Get the beat for `self`.
    pub fn beat(&self) -> u32 {
        self.beat.trunc() as u32
    }

    /// Get the sub beat for `self`.
    pub fn sub_beat(&self) -> f64 {
        self.beat.fract()
    }
}

impl std::ops::Add for Position {
    type Output = Position;

    fn add(self, rhs: Position) -> Position {
        Position {
            beat: self.beat + rhs.beat,
        }
    }
}

impl std::ops::AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_value_is_zero() {
        assert_eq!(Position::default(), Position::new(0.0));
        assert_eq!(Position::default().beat(), 0);
        assert_eq!(Position::default().sub_beat(), 0.0);
    }

    #[test]
    fn new_beat_position_with_has_beat_and_sub_beat() {
        let p = Position::new(11.5);
        assert_eq!(p.beat(), 11);
        assert_eq!(p.sub_beat(), 0.5);
    }

    #[test]
    fn add_beat_adds_components_and_carries_the_sub_beat() {
        assert_eq!(
            Position::new(1.625) + Position::new(3.75),
            Position::new(5.375)
        );
    }
}
