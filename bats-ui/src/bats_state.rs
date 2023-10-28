use bats_async::{Command, CommandSender};
use bats_dsp::{buffers::Buffers, SampleRate};
use bats_lib::{
    plugin::{toof::Toof, BatsInstrument},
    Bats, Track,
};

/// Contains state for dealing with
pub struct BatsState {
    /// Used to send commands to bats.
    commands: CommandSender,
    /// The current BPM.
    bpm: f32,
    /// The current BPM as a string.
    bpm_text: String,
    /// Details for the current tracks.
    tracks: Vec<TrackDetails>,
    /// The sample rate.
    pub sample_rate: SampleRate,
    /// The buffer size.
    buffer_size: usize,
    /// The next unique id.
    next_id: u32,
}

/// Contains track details.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackDetails {
    pub id: u32,
    pub name: &'static str,
}

impl TrackDetails {
    /// Create a new `PluginDetails` from a `PluginInstance`.
    fn new(p: &Track) -> TrackDetails {
        TrackDetails {
            id: p.id,
            name: p.plugin.name(),
        }
    }
}

impl BatsState {
    /// Create a new `BatsState`.
    pub fn new(bats: &Bats, commands: CommandSender) -> BatsState {
        let bpm = bats.metronome.bpm();
        let next_id = bats.tracks.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        BatsState {
            commands,
            bpm,
            bpm_text: format_bpm(bpm),
            tracks: bats.tracks.iter().map(TrackDetails::new).collect(),
            sample_rate: bats.sample_rate,
            buffer_size: bats.buffer_size,
            next_id,
        }
    }

    /// Take the next unique id.
    fn take_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Add a new plugin.
    pub fn add_plugin(&mut self, plugin: Toof) -> &TrackDetails {
        let id = self.take_id();
        let plugin = Track {
            id,
            plugin,
            output: Buffers::new(self.buffer_size),
        };
        self.tracks.push(TrackDetails::new(&plugin));
        self.commands.send(Command::AddTrack(plugin));
        self.tracks.last().unwrap()
    }

    /// Set the armed plugin by id.
    pub fn set_armed(&mut self, armed: Option<u32>) {
        self.commands.send(Command::SetArmedTrack(armed));
    }

    /// Set the bpm.
    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm;
        self.bpm_text = format_bpm(bpm);
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

    /// Get all the tracks.
    pub fn tracks(&self) -> impl '_ + Iterator<Item = &TrackDetails> {
        self.tracks.iter()
    }
}

fn format_bpm(bpm: f32) -> String {
    format!("{:.1}", bpm)
}
