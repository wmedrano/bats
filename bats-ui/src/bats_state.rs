use std::{cell::RefCell, collections::HashMap};

use bats_async::{command::Command, notification::Notification, CommandSender};
use bats_dsp::sample_rate::SampleRate;
use bats_lib::{
    plugin::{metadata::Metadata, MidiEvent},
    plugin_factory::AnyPlugin,
    track::Track,
    Bats,
};
use log::{error, info};

/// Contains state for dealing with
pub struct BatsState {
    /// The sample rate.
    sample_rate: SampleRate,
    /// The buffer size.
    buffer_size: usize,
    /// Used to send commands to bats.
    commands: CommandSender,
    /// The inner state.
    state: RefCell<InnerState>,
}

struct InnerState {
    /// The armed track.
    armed_track: usize,
    /// True if recording is enabled.
    recording_enabled: bool,
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
        let plugin_metadata = t.plugin.plugin().metadata();
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
        BatsState {
            commands,
            sample_rate: bats.sample_rate,
            buffer_size: bats.buffer_size,
            state: InnerState::new(bats).into(),
        }
    }

    /// Handle all notifications.
    pub fn handle_notifications(&self) {
        for notification in self.commands.notifications() {
            match notification {
                Notification::Undo(_) => {
                    // TODO: Implement undo functionality.
                }
            }
        }
    }

    /// Get the sample rate.
    pub fn sample_rate(&self) -> SampleRate {
        self.handle_notifications();
        self.sample_rate
    }

    /// Get the buffer size.
    pub fn buffer_size(&self) -> usize {
        self.handle_notifications();
        self.buffer_size
    }

    /// Set the plugin for the track.
    pub fn set_plugin(&self, track_id: usize, plugin: AnyPlugin) {
        self.handle_notifications();
        info!(
            "Setting track {track_id} plugin to {plugin_name}.",
            plugin_name = plugin.plugin().metadata().name
        );
        match self.state.borrow_mut().tracks.get_mut(track_id) {
            None => {
                error!("Could not find track with id {track_id}.");
            }
            Some(track) => {
                track.plugin_metadata = plugin.plugin().metadata();
                track.params = param_values(&plugin);
                self.commands.send(Command::SetPlugin { track_id, plugin });
            }
        }
    }

    /// Return the currently armed track.
    pub fn armed(&self) -> usize {
        self.handle_notifications();
        self.state.borrow().armed_track
    }

    /// Set the armed plugin by id.
    pub fn set_armed(&self, armed: usize) {
        self.handle_notifications();
        let mut state = self.state.borrow_mut();
        if state.armed_track == armed {
            return;
        }
        state.armed_track = armed;
        self.commands.send(Command::SetArmedTrack(armed));
    }

    /// True if recording is enabled.
    pub fn recording_enabled(&self) -> bool {
        self.handle_notifications();
        self.state.borrow().recording_enabled
    }

    /// Toggle if recording is enabled.
    pub fn toggle_recording(&self) {
        let enabled = !self.state.borrow().recording_enabled;
        self.set_recording(enabled);
    }

    /// Set if recording is enabled.
    pub fn set_recording(&self, enabled: bool) {
        self.handle_notifications();
        let mut state = self.state.borrow_mut();
        if state.recording_enabled == enabled {
            return;
        }
        state.recording_enabled = enabled;
        self.commands.send(Command::SetRecord(enabled));
    }

    /// Set the track volume.
    pub fn modify_track_volume(&self, track_id: usize, f: impl Fn(&TrackDetails) -> f32) {
        self.handle_notifications();
        if let Some(t) = self.state.borrow_mut().tracks.get_mut(track_id) {
            t.volume = f(t).clamp(0.00796, 4.0);
            self.commands.send(Command::SetTrackVolume {
                track_id,
                volume: t.volume,
            });
        }
    }

    /// Modify the bpm.
    pub fn modify_bpm(&self, f: impl Fn(f32) -> f32) {
        self.handle_notifications();
        let mut state = self.state.borrow_mut();
        state.bpm = f(state.bpm).clamp(10.0, 360.0);
        self.commands.send(Command::SetTransportBpm(state.bpm));
    }

    /// The current BPM.
    pub fn bpm(&self) -> f32 {
        self.handle_notifications();
        self.state.borrow().bpm
    }

    /// Modify the metronome volume.
    pub fn modify_metronome(&self, f: impl Fn(f32) -> f32) {
        self.handle_notifications();
        let mut state = self.state.borrow_mut();
        let v = f(state.metronome_volume).clamp(0.0, 1.0);
        state.metronome_volume = v;
        self.commands
            .send(Command::SetMetronomeVolume(state.metronome_volume));
    }

    /// Get the metronome volume.
    pub fn metronome_volume(&self) -> f32 {
        self.handle_notifications();
        self.state.borrow().metronome_volume
    }

    /// Get all the tracks.
    pub fn tracks_vec(&self) -> Vec<TrackDetails> {
        self.handle_notifications();
        self.state.borrow().tracks.to_vec()
    }

    /// Get a track by its id.
    pub fn track_by_id(&self, track_id: usize) -> Option<TrackDetails> {
        self.handle_notifications();
        self.state.borrow().tracks.get(track_id).cloned()
    }

    /// Get the param value for the given `param_id` for `track_id`.
    pub fn param(&self, track_id: usize, param_id: u32) -> f32 {
        self.handle_notifications();
        let state = self.state.borrow();
        let track = match state.tracks.get(track_id) {
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
    pub fn modify_param(&self, track_id: usize, param_id: u32, f: impl Fn(f32) -> f32) {
        self.handle_notifications();
        let mut state = self.state.borrow_mut();
        let track = match state.tracks.get_mut(track_id) {
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

    /// Set the sequence for the track.
    pub fn set_sequence(&self, track_id: usize, mut sequence: Vec<MidiEvent>) {
        self.handle_notifications();
        sequence.reserve(Track::SEQUENCE_CAPACITY);
        self.commands
            .send(Command::SetSequence { track_id, sequence });
    }
}

impl InnerState {
    /// Create a new `InnerState`.
    pub fn new(bats: &Bats) -> InnerState {
        let bpm = bats.transport.bpm();
        let tracks = core::array::from_fn(|idx| TrackDetails::new(idx, &bats.tracks[idx]));
        InnerState {
            armed_track: bats.armed_track,
            recording_enabled: bats.recording_enabled,
            bpm,
            metronome_volume: bats.transport.metronome_volume,
            tracks,
        }
    }
}

/// Get the map from `param_id` to the parameter value.
fn param_values(p: &AnyPlugin) -> HashMap<u32, f32> {
    let p = p.plugin();
    p.metadata()
        .params
        .iter()
        .map(|param| {
            let value = p.param(param.id);
            (param.id, value)
        })
        .collect()
}
