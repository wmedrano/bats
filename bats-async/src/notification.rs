use bats_lib::Bats;

use crate::command::Command;

#[derive(Clone, Debug, PartialEq)]
/// A notification for the UI.
pub enum Notification {
    /// Notify that a new undo command is available.
    Undo(Command),
    /// Notify that save is ready.
    SaveResponse(Box<Bats>),
    /// Notify that a save has been loaded. The previous state is returned.
    SaveLoaded { old: Box<Bats> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_size_is_reasonable() {
        let size = std::mem::size_of::<Notification>();
        assert_eq!(size, 40);
    }
}
