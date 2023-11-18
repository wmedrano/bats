use crate::command::Command;

#[derive(Clone, Debug, PartialEq)]
/// A notification for the UI.
pub enum Notification {
    /// Notify that a new undo command is available.
    Undo(Command),
}
