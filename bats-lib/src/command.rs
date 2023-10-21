use crossbeam_channel::{Receiver, Sender};

use crate::Bats;

/// Contains commands for bats.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Command {
    /// Set the metronome volume.
    SetMetronomeVolume(f32),
}

/// Send commands to a bats instance.
#[derive(Clone, Debug)]
pub struct CommandSender {
    sender: Sender<Command>,
}

/// Receive commands for a bats instance.
#[derive(Clone, Debug)]
pub struct CommandReceiver {
    receiver: Receiver<Command>,
}

/// Create a new `CommandSender` and `CommandReceiver`.
pub fn new_async_commander() -> (CommandSender, CommandReceiver) {
    let (sender, receiver) = crossbeam_channel::bounded(1024);
    (CommandSender { sender }, CommandReceiver { receiver })
}

impl CommandSender {
    /// Send a single command.
    pub fn send(&self, cmd: Command) {
        self.sender.send(cmd).unwrap();
    }
}

impl CommandReceiver {
    /// Execute all queued up commands.
    pub fn execute_all(&mut self, b: &mut Bats) {
        for cmd in self.receiver.try_iter() {
            cmd.execute(b);
        }
    }
}

impl Command {
    /// The command to execute. It returns the command to undo the current command.
    pub fn execute(self, b: &mut Bats) -> Command {
        match self {
            Command::SetMetronomeVolume(v) => {
                let undo = Command::SetMetronomeVolume(b.metronome_volume);
                b.metronome_volume = v;
                undo
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_metronome_volume_sets_volume() {
        let mut b = Bats::new(44100.0, 64);
        b.metronome_volume = 1.0;

        let undo = Command::SetMetronomeVolume(0.5).execute(&mut b);
        assert_eq!(b.metronome_volume, 0.5);
        assert_eq!(undo, Command::SetMetronomeVolume(1.0));
    }
}
