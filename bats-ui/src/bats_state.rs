use std::collections::HashMap;

use bats_async::{command::Command, CommandSender};
use bats_dsp::{buffers::Buffers, sample_rate::SampleRate};
use bats_lib::{
    plugin::{metadata::Metadata, toof::Toof, BatsInstrument},
    Bats, Track,
};
use log::error;

/// Contains state for dealing with
pub struct BatsState {
    /// Used to send commands to bats.
    commands: CommandSender,
    /// The armed track.
    armed_track: Option<u32>,
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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackDetails {
    pub id: u32,
    pub plugin_metadata: &'static Metadata,
    pub volume: f32,
    pub params: HashMap<u32, f32>,
}

impl TrackDetails {
    /// Create a new `PluginDetails` from a `PluginInstance`.
    fn new(t: &Track) -> TrackDetails {
        let plugin_metadata = t.plugin.metadata();
        let params = plugin_metadata
            .params
            .iter()
            .map(|p| (p.id, p.default_value))
            .collect();
        TrackDetails {
            id: t.id,
            plugin_metadata,
            volume: t.volume,
            params,
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
            armed_track: None,
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
            volume: 1.0,
            output: Buffers::new(self.buffer_size),
        };
        self.tracks.push(TrackDetails::new(&plugin));
        self.commands.send(Command::AddTrack(plugin));
        self.tracks.last().unwrap()
    }

    /// Return the currently armed track.
    pub fn armed(&mut self) -> Option<u32> {
        self.armed_track
    }

    /// Set the armed plugin by id.
    pub fn set_armed(&mut self, armed: Option<u32>) {
        self.armed_track = armed;
        self.commands.send(Command::SetArmedTrack(armed));
    }

    pub fn set_track_volume(&mut self, track_id: u32, volume: f32) {
        if let Some(t) = self.tracks.iter_mut().find(|t| t.id == track_id) {
            t.volume = volume;
            self.commands
                .send(Command::SetTrackVolume { track_id, volume });
        }
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

    /// Get the currently selected track.
    pub fn selected_track(&self) -> Option<&TrackDetails> {
        self.tracks.iter().find(|t| Some(t.id) == self.armed_track)
    }

    pub fn param(&self, track_id: u32, param_id: u32) -> f32 {
        match self.tracks.iter().find(|t| t.id == track_id) {
            None => 0.0,
            Some(t) => t.params.get(&param_id).copied().unwrap_or(0.0),
        }
    }

    pub fn set_param(&mut self, track_id: u32, param_id: u32, value: f32) {
        match self.tracks.iter_mut().find(|t| t.id == track_id) {
            None => error!("Could not find track {track_id} to set param {param_id} to {value}"),
            Some(t) => {
                t.params.insert(param_id, value);
                self.commands.send(Command::SetParam {
                    track_id,
                    param_id,
                    value,
                });
            }
        }
    }
}

fn format_bpm(bpm: f32) -> String {
    format!("{:.1}", bpm)
}
