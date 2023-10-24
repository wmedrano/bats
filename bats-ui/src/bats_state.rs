use bats_async::{Command, CommandSender};
use bats_dsp::{Buffers, SampleRate};
use bats_lib::{
    plugin::{toof::Toof, BatsInstrument},
    Bats, PluginInstance,
};

/// Contains state for dealing with
pub struct BatsState {
    /// Used to send commands to bats.
    commands: CommandSender,
    /// The current BPM.
    bpm: f32,
    /// The current BPM as a string.
    bpm_text: String,
    /// The name of the current plugins.
    plugins: Vec<PluginDetails>,
    /// The sample rate.
    pub sample_rate: SampleRate,
    /// The buffer size.
    buffer_size: usize,
    /// The next plugin id.
    next_id: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PluginDetails {
    id: u32,
    name: &'static str,
}

impl PluginDetails {
    fn new(p: &PluginInstance) -> PluginDetails {
        PluginDetails {
            id: p.id,
            name: p.plugin.name(),
        }
    }
}

impl BatsState {
    pub fn new(bats: &Bats, commands: CommandSender) -> BatsState {
        let bpm = bats.metronome.bpm();
        let next_id = bats.plugins.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        BatsState {
            commands,
            bpm,
            bpm_text: bpm.to_string(),
            plugins: bats.plugins.iter().map(PluginDetails::new).collect(),
            sample_rate: bats.sample_rate,
            buffer_size: bats.buffer_size,
            next_id,
        }
    }

    fn take_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add_plugin(&mut self, plugin: Toof) {
        let id = self.take_id();
        let plugin = PluginInstance {
            id,
            plugin,
            output: Buffers::new(self.buffer_size),
        };
        self.plugins.push(PluginDetails::new(&plugin));
        self.commands.send(Command::AddPlugin(plugin));
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
        self.plugins.iter().map(|p| p.name)
    }
}
