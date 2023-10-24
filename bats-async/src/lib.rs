use bats_lib::{Bats, PluginInstance};
use crossbeam_channel::{Receiver, Sender};
use log::info;

const DEFAULT_METRONOME_VOLUME: f32 = 0.8;

/// Contains commands for bats.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// No command.
    None,
    /// Toggle the metrenome.
    ToggleMetronome,
    /// Set the BPM of the metronome.
    SetMetronomeBpm(f32),
    /// Add a new plugin.
    AddPlugin(PluginInstance),
    /// Remove a plugin.
    RemovePlugin { id: u32 },
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
        info!("Sending command: {:?}", cmd);
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
            Command::None => Command::None,
            Command::ToggleMetronome => {
                if b.metronome_volume > 0.0 {
                    b.metronome_volume = 0.0;
                } else {
                    b.metronome_volume = DEFAULT_METRONOME_VOLUME;
                }
                Command::ToggleMetronome
            }
            Command::SetMetronomeBpm(bpm) => {
                let previous_bpm = b.metronome.bpm();
                b.metronome.set_bpm(b.sample_rate, bpm);
                Command::SetMetronomeBpm(previous_bpm)
            }
            Command::AddPlugin(plugin) => {
                let id = plugin.id;
                b.add_plugin(plugin);
                Command::RemovePlugin { id }
            }
            Command::RemovePlugin { id } => match b.plugins.iter().position(|p| p.id == id) {
                None => Command::None,
                Some(idx) => Command::AddPlugin(b.plugins.remove(idx)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use bats_dsp::SampleRate;

    use super::*;

    #[test]
    fn toggle_metronome_volume_toggles_volume() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        b.metronome_volume = DEFAULT_METRONOME_VOLUME;

        let undo = Command::ToggleMetronome.execute(&mut b);
        assert_eq!(b.metronome_volume, 0.0);
        assert_eq!(undo, Command::ToggleMetronome);

        let undo = Command::ToggleMetronome.execute(&mut b);
        assert_eq!(b.metronome_volume, DEFAULT_METRONOME_VOLUME);
        assert_eq!(undo, Command::ToggleMetronome);
    }

    #[test]
    fn metrenome_set_bpm() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        b.metronome.set_bpm(b.sample_rate, 100.0);

        let undo = Command::SetMetronomeBpm(90.0).execute(&mut b);
        assert_eq!(b.metronome.bpm(), 90.0);
        assert_eq!(undo, Command::SetMetronomeBpm(100.0));
    }
}
