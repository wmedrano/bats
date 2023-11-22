use bats_lib::Bats;
use command::Command;
use crossbeam_channel::{Receiver, Sender};
use log::{error, info};
use notification::Notification;

pub mod command;
pub mod notification;

/// Send commands to a bats instance.
pub struct CommandSender {
    /// The channel to send commands to.
    sender: Sender<Command>,
    /// Then channel to receive notifications from.
    notifications: Receiver<Notification>,
}

/// Receive commands for a bats instance.
#[derive(Debug)]
pub struct CommandReceiver {
    /// The channel to receive commands from.
    receiver: Receiver<Command>,
    /// The channel to send notifications to.
    notifications: Sender<Notification>,
}

/// Create a new `CommandSender` and `CommandReceiver`.
pub fn new_async_commander() -> (CommandSender, CommandReceiver) {
    let (sender, receiver) = crossbeam_channel::bounded(1024);
    let (n_sender, n_receiver) = crossbeam_channel::bounded(1024);
    (
        CommandSender {
            sender,
            notifications: n_receiver,
        },
        CommandReceiver {
            receiver,
            notifications: n_sender,
        },
    )
}

impl CommandSender {
    /// Send a single command.
    pub fn send(&self, cmd: Command) {
        info!("Sending command: {:?}", cmd);
        self.sender.send(cmd).unwrap();
    }

    /// Get all pending notifications
    pub fn notifications(&self) -> Vec<Notification> {
        self.notifications.try_iter().collect()
    }
}

impl CommandReceiver {
    /// Execute all queued up commands and return an iterator of the undo commands.
    pub fn execute_all<'a>(&'a self, b: &'a mut Bats) {
        for cmd in self.receiver.try_iter() {
            let undo = cmd.execute(b);
            if let Err(err) = self.notifications.try_send(Notification::Undo(undo)) {
                error!("Failed to send undo notifcation: {err}");
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bats_dsp::sample_rate::SampleRate;
    use bats_lib::{
        builder::{AnyPlugin, BatsBuilder},
        plugin::{empty::Empty, toof::Toof},
    };

    #[test]
    fn send_commands_get_executed() {
        let (sender, receiver) = new_async_commander();
        let mut bats = BatsBuilder {
            sample_rate: SampleRate::new(44100.0),
            buffer_size: 64,
            bpm: 120.0,
            tracks: Default::default(),
        }
        .build();
        let plugin = AnyPlugin::Toof(Toof::new(bats.sample_rate));
        assert_eq!(bats.tracks[0].plugin, AnyPlugin::Empty(Empty));
        assert_eq!(sender.notifications(), vec![]);
        sender.send(Command::None);
        sender.send(Command::SetPlugin {
            track_id: 0,
            plugin: plugin.clone(),
        });

        receiver.execute_all(&mut bats);
        assert_eq!(
            sender.notifications(),
            vec![
                Notification::Undo(Command::None),
                Notification::Undo(Command::SetPlugin {
                    track_id: 0,
                    plugin: AnyPlugin::Empty(Empty),
                })
            ]
        );
        assert_eq!(bats.tracks[0].plugin, plugin);
    }
}
