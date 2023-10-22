use bats_async::{Command, CommandSender};

/// Contains state for dealing with
pub struct BatsState {
    /// Used to send commands to bats.
    commands: CommandSender,
    /// The current BPM.
    bpm: f32,
    /// The current BPM as a string.
    bpm_text: String,
    /// The name of the current plugins.
    plugin_names: Vec<&'static str>,
}

impl BatsState {
    pub fn new(commands: CommandSender, plugin_names: Vec<&'static str>) -> BatsState {
        let bpm = 120.0;
        commands.send(Command::SetMetronomeBpm(bpm));
        BatsState {
            commands,
            bpm,
            bpm_text: bpm.to_string(),
            plugin_names,
        }
    }

    /// Set the bpm.
    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm;
        self.bpm_text = bpm.to_string();
        self.commands.send(Command::SetMetronomeBpm(bpm));
    }

    // The current BPM.
    pub fn bpm(&self) -> f32 {
        self.bpm
    }

    /// The current BPM as text.
    pub fn bpm_text(&self) -> &str {
        &self.bpm_text
    }

    /// Toggle the metronome.
    pub fn toggle_metronome(&self) {
        self.commands.send(Command::ToggleMetronome);
    }

    /// Get the set of plugin names.
    pub fn plugin_names(&self) -> impl '_ + Iterator<Item = &'static str> {
        self.plugin_names.iter().copied()
    }
}
