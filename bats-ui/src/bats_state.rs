use std::collections::HashMap;

use bats_async::{command::Command, CommandSender};
use bats_dsp::sample_rate::SampleRate;
use bats_lib::{
    plugin::{metadata::Metadata, toof::Toof, BatsInstrument},
    track::Track,
    Bats,
};
use log::{error, info};

/// Contains state for dealing with
pub struct BatsState {
    /// The sample rate.
    pub sample_rate: SampleRate,
    /// The buffer size.
    pub buffer_size: usize,
    /// Used to send commands to bats.
    commands: CommandSender,
    /// The armed track.
    armed_track: usize,
    /// The current BPM.
    bpm: f32,
    /// The volume of the metronome.
    metronome_volume: f32,
    /// Details for all the tracks.
    tracks: [TrackDetails; Bats::SUPPORTED_TRACKS],
}

/// Contains track details.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackDetails {
    pub id: usize,
    pub plugin_metadata: &'static Metadata,
    pub volume: f32,
    pub params: HashMap<u32, f32>,
}

impl Default for TrackDetails {
    /// Create a placeholder entry for `TrackDetails`.
    fn default() -> TrackDetails {
        TrackDetails {
            id: 0,
            plugin_metadata: &Metadata {
                name: "default_plugin",
                params: &[],
            },
            volume: 1.0,
            params: HashMap::new(),
        }
    }
}

impl TrackDetails {
    /// Create a new `PluginDetails` from a `PluginInstance`.
    fn new(id: usize, t: &Track) -> TrackDetails {
        let plugin_metadata = plugin_metadata(&t.plugin);
        let params = param_values(&t.plugin);
        TrackDetails {
            id,
            plugin_metadata,
            volume: t.volume,
            params,
        }
    }

    /// Return the human readable title of the track.
    pub fn title(&self) -> String {
        format!(
            "{track_number} - {plugin_name}",
            track_number = self.id + 1,
            plugin_name = self.plugin_metadata.name
        )
    }
}

impl BatsState {
    /// Create a new `BatsState`.
    pub fn new(bats: &Bats, commands: CommandSender) -> BatsState {
        let bpm = bats.transport.bpm();
        let tracks = core::array::from_fn(|idx| TrackDetails::new(idx, &bats.tracks[idx]));
        let armed_track = bats.armed_track;
        BatsState {
            commands,
            armed_track,
            bpm,
            metronome_volume: bats.transport.metronome_volume,
            tracks,
            sample_rate: bats.sample_rate,
            buffer_size: bats.buffer_size,
        }
    }

    /// Set the plugin for the track.
    pub fn set_plugin(&mut self, track_id: usize, plugin: Option<Box<Toof>>) {
        info!(
            "Setting track {track_id} plugin to {plugin_name}.",
            plugin_name = plugin.as_ref().map(|p| p.metadata().name).unwrap_or("")
        );
        match self.tracks.get_mut(track_id) {
            None => {
                error!("Could not find track with id {track_id}.");
            }
            Some(track) => {
                track.plugin_metadata = plugin_metadata(&plugin);
                track.params = param_values(&plugin);
                self.commands.send(Command::SetPlugin { track_id, plugin });
            }
        }
    }

    /// Return the currently armed track.
    pub fn armed(&mut self) -> usize {
        self.armed_track
    }

    /// Set the armed plugin by id.
    pub fn set_armed(&mut self, armed: usize) {
        self.armed_track = armed;
        self.commands.send(Command::SetArmedTrack(self.armed_track));
    }

    /// Set the track volume.
    pub fn modify_track_volume(&mut self, track_id: usize, f: impl Fn(&TrackDetails) -> f32) {
        if let Some(t) = self.tracks.get_mut(track_id) {
            t.volume = f(t).clamp(0.00796, 4.0);
            self.commands.send(Command::SetTrackVolume {
                track_id,
                volume: t.volume,
            });
        }
    }

    /// Modify the bpm.
    pub fn modify_bpm(&mut self, f: impl Fn(f32) -> f32) {
        self.bpm = f(self.bpm);
        self.commands.send(Command::SetTransportBpm(self.bpm));
    }

    // The current BPM.
    pub fn bpm(&self) -> f32 {
        self.bpm
    }

    /// Modify the metronome volume.
    pub fn modify_metronome(&mut self, f: impl Fn(f32) -> f32) {
        let v = f(self.metronome_volume).clamp(0.0, 1.0);
        self.metronome_volume = v;
        self.commands
            .send(Command::SetMetronomeVolume(self.metronome_volume));
    }

    /// Get the metronome volume.
    pub fn metronome_volume(&self) -> f32 {
        self.metronome_volume
    }

    /// Get all the tracks.
    pub fn tracks(&self) -> &[TrackDetails; Bats::SUPPORTED_TRACKS] {
        &self.tracks
    }

    /// Get a track by its id.
    pub fn track_by_id(&self, track_id: usize) -> Option<&TrackDetails> {
        self.tracks.get(track_id)
    }

    /// Get the param value for the given `param_id` for `track_id`.
    pub fn param(&self, track_id: usize, param_id: u32) -> f32 {
        let track = match self.tracks.get(track_id) {
            Some(t) => t,
            None => {
                error!("Attempted to get track for invalid track id {track_id}.");
                return 0.0;
            }
        };
        let get_default_value = || match track.plugin_metadata.param_by_id(param_id) {
            None => 0.0,
            Some(p) => p.default_value,
        };
        track
            .params
            .get(&param_id)
            .copied()
            .unwrap_or_else(get_default_value)
    }

    /// Modify the param value by applying `f`. This function also handles keeping the param valid,
    /// like adjusting according to the min and max values.
    pub fn modify_param(&mut self, track_id: usize, param_id: u32, f: impl Fn(f32) -> f32) {
        let track = match self.tracks.get_mut(track_id) {
            None => {
                error!("Could not find track {track_id} to modify param {param_id}.");
                return;
            }
            Some(t) => t,
        };
        let param = match track.plugin_metadata.param_by_id(param_id) {
            Some(p) => p,
            None => {
                error!(
                    "Could not find param with id {param_id} for track with plugin {plugin_name}.",
                    plugin_name = track.plugin_metadata.name
                );
                return;
            }
        };
        let current_value = *track.params.get(&param_id).unwrap();
        let value = f(current_value).clamp(param.min_value, param.max_value);
        track.params.insert(param_id, value);
        self.commands.send(Command::SetParam {
            track_id,
            param_id,
            value,
        });
    }
}

/// Get the map from `param_id` to the parameter value.
fn param_values(p: &Option<Box<Toof>>) -> HashMap<u32, f32> {
    plugin_metadata(p)
        .params
        .iter()
        .map(|param| {
            let value = p
                .as_ref()
                .map(|plugin| plugin.param(param.id))
                .unwrap_or(param.default_value);
            (param.id, value)
        })
        .collect()
}

/// Get the metadata for the given plugin.
fn plugin_metadata(p: &Option<Box<Toof>>) -> &'static Metadata {
    p.as_ref().map(|p| p.metadata()).unwrap_or(&Metadata {
        name: "empty",
        params: &[],
    })
}
