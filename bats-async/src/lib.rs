use bats_lib::Bats;
use command::Command;
use crossbeam_channel::{Receiver, Sender};
use log::info;

pub mod command;

/// Send commands to a bats instance.
#[derive(Clone, Debug)]
pub struct CommandSender {
    sender: Sender<Command>,
}

/// Receive commands for a bats instance.
#[derive(Clone, Debug)]
pub struct CommandReceiver {
    /// The channel to receive commands from.
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
        info!("Sending command: {:?}", cmd);
        self.sender.send(cmd).unwrap();
    }
}

impl CommandReceiver {
    /// Execute all queued up commands and return an iterator of the undo commands.
    pub fn execute_all<'a>(&'a mut self, b: &'a mut Bats) -> impl 'a + Iterator<Item = Command> {
        self.receiver.try_iter().map(|cmd| cmd.execute(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bats_dsp::sample_rate::SampleRate;
    use bats_lib::{plugin::toof::Toof, Bats};

    #[test]
    fn send_commands_get_executed() {
        let (sender, mut receiver) = new_async_commander();
        let mut bats = Bats::new(SampleRate::new(44100.0), 64);
        let plugin = Some(Toof::new(bats.sample_rate));
        assert_eq!(bats.tracks[0].plugin, None);
        sender.send(Command::None);
        sender.send(Command::SetPlugin {
            track_id: 0,
            plugin: plugin.clone(),
        });
        assert_eq!(
            receiver.execute_all(&mut bats).collect::<Vec<_>>(),
            vec![
                Command::None,
                Command::SetPlugin {
                    track_id: 0,
                    plugin: None
                }
            ]
        );
        assert_eq!(bats.tracks[0].plugin, plugin);
    }
}
