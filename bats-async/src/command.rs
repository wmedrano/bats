use bats_lib::{plugin::MidiEvent, plugin_factory::AnyPlugin, Bats};
use log::error;

/// Contains commands for bats.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// No command.
    None,
    /// Set the metrenome.
    SetMetronomeVolume(f32),
    /// Set the BPM of the transport.
    SetTransportBpm(f32),
    /// Add a new track.
    SetPlugin { track_id: usize, plugin: AnyPlugin },
    /// Set the armed track.
    SetArmedTrack(usize),
    /// Set the track volume.
    SetTrackVolume { track_id: usize, volume: f32 },
    /// Set a parameter.
    SetParam {
        track_id: usize,
        param_id: u32,
        value: f32,
    },
    /// Set the sequence for the track.
    SetSequence {
        track_id: usize,
        sequence: Vec<MidiEvent>,
    },
    /// Set if recording is enabled or disabled.
    SetRecord(bool),
}

impl Command {
    /// The command to execute. It returns the command to undo the current command.
    pub fn execute(self, b: &mut Bats) -> Command {
        match self {
            Command::None => Command::None,
            Command::SetMetronomeVolume(v) => {
                let old = b.transport.metronome_volume;
                b.transport.metronome_volume = v;
                Command::SetMetronomeVolume(old)
            }
            Command::SetTransportBpm(bpm) => {
                let previous_bpm = b.transport.bpm();
                b.transport.set_bpm(b.sample_rate, bpm);
                Command::SetTransportBpm(previous_bpm)
            }
            Command::SetPlugin { track_id, plugin } => match b.tracks.get_mut(track_id) {
                None => Command::None,
                Some(t) => {
                    let mut old_plugin = plugin;
                    std::mem::swap(&mut t.plugin, &mut old_plugin);
                    Command::SetPlugin {
                        track_id,
                        plugin: old_plugin,
                    }
                }
            },
            Command::SetTrackVolume { track_id, volume } => match b.tracks.get_mut(track_id) {
                None => Command::None,
                Some(t) => {
                    let undo = Command::SetTrackVolume {
                        track_id,
                        volume: t.volume,
                    };
                    t.volume = volume;
                    undo
                }
            },
            Command::SetArmedTrack(armed) => {
                let undo = Command::SetArmedTrack(b.armed_track);
                b.armed_track = armed;
                undo
            }
            Command::SetParam {
                track_id,
                param_id,
                value,
            } => match b.tracks.get_mut(track_id) {
                Some(t) => {
                    let p = t.plugin.plugin_mut();
                    let undo = Command::SetParam {
                        track_id,
                        param_id,
                        value: p.param(param_id),
                    };
                    p.set_param(param_id, value);
                    undo
                }
                None => {
                    error!(
                        "track {} does not exist, will not set param {} to {}.",
                        track_id, param_id, value
                    );
                    Command::None
                }
            },
            Command::SetSequence {
                track_id,
                mut sequence,
            } => match b.tracks.get_mut(track_id) {
                Some(t) => {
                    std::mem::swap(&mut sequence, &mut t.sequence);
                    Command::SetSequence { track_id, sequence }
                }
                None => {
                    error!("track {track_id} does not exist, will not clear the sequence.");
                    Command::None
                }
            },
            Command::SetRecord(enabled) => {
                let undo = Command::SetRecord(b.recording_enabled);
                b.recording_enabled = enabled;
                undo
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bats_dsp::{position::Position, sample_rate::SampleRate};
    use bats_lib::plugin::{empty::Empty, toof::Toof};
    use bmidi::MidiMessage;

    use super::*;

    /// Get all the track name for non-empty tracks.
    fn get_track_names(b: &Bats) -> Vec<&'static str> {
        b.tracks
            .iter()
            .map(|t| t.plugin.plugin().metadata().name)
            .collect()
    }

    #[test]
    fn command_size_is_reasonable() {
        let size = std::mem::size_of::<Command>();
        assert_eq!(size, 40);
    }

    #[test]
    fn none_command_undo_is_none() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        let undo = Command::None.execute(&mut b);
        assert_eq!(undo, Command::None);
    }

    #[test]
    fn set_metronome_volume_sets_new_volume_and_returns_old_as_undo() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        b.transport.metronome_volume = 1.0;

        let undo = Command::SetMetronomeVolume(0.5).execute(&mut b);
        assert_eq!(b.transport.metronome_volume, 0.5);
        assert_eq!(undo, Command::SetMetronomeVolume(1.0));
    }

    #[test]
    fn metrenome_set_bpm() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        b.transport.set_bpm(b.sample_rate, 100.0);

        let undo = Command::SetTransportBpm(90.0).execute(&mut b);
        assert_eq!(b.transport.bpm(), 90.0);
        assert_eq!(undo, Command::SetTransportBpm(100.0));
    }

    #[test]
    fn set_plugin() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        let plugin = AnyPlugin::Toof(Toof::new(b.sample_rate));
        b.tracks[0].plugin = plugin.clone();
        assert_eq!(
            get_track_names(&b),
            vec!["toof", "empty", "empty", "empty", "empty", "empty", "empty", "empty"]
        );

        let undo = Command::SetPlugin {
            track_id: 1,
            plugin: plugin.clone(),
        }
        .execute(&mut b);
        assert_eq!(
            get_track_names(&b),
            vec!["toof", "toof", "empty", "empty", "empty", "empty", "empty", "empty"]
        );
        assert_eq!(
            undo,
            Command::SetPlugin {
                track_id: 1,
                plugin: AnyPlugin::Empty(Empty)
            }
        );

        let undo = Command::SetPlugin {
            track_id: 1,
            plugin: AnyPlugin::Empty(Empty),
        }
        .execute(&mut b);
        assert_eq!(
            get_track_names(&b),
            vec!["toof", "empty", "empty", "empty", "empty", "empty", "empty", "empty"]
        );
        assert_eq!(
            undo,
            Command::SetPlugin {
                track_id: 1,
                plugin: plugin.clone()
            }
        );
    }

    #[test]
    fn remove_plugin_that_does_not_exist_does_nothing() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        let plugin = AnyPlugin::Toof(Toof::new(b.sample_rate));
        b.tracks[0].plugin = plugin.clone();
        b.tracks[2].plugin = plugin.clone();
        assert_eq!(
            get_track_names(&b),
            vec!["toof", "empty", "toof", "empty", "empty", "empty", "empty", "empty"]
        );
        assert_eq!(
            Command::SetPlugin {
                track_id: 1,
                plugin: AnyPlugin::Empty(Empty),
            }
            .execute(&mut b),
            Command::SetPlugin {
                track_id: 1,
                plugin: AnyPlugin::default()
            }
        );
        assert_eq!(
            get_track_names(&b),
            vec!["toof", "empty", "toof", "empty", "empty", "empty", "empty", "empty"]
        );
    }

    #[test]
    fn set_armed_track() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        b.armed_track = 100;

        let undo = Command::SetArmedTrack(10).execute(&mut b);
        assert_eq!(b.armed_track, 10);
        assert_eq!(undo, Command::SetArmedTrack(100));

        let undo = Command::SetArmedTrack(20).execute(&mut b);
        assert_eq!(b.armed_track, 20);
        assert_eq!(undo, Command::SetArmedTrack(10));

        let undo = Command::SetArmedTrack(100).execute(&mut b);
        assert_eq!(b.armed_track, 100);
        assert_eq!(undo, Command::SetArmedTrack(20));
    }

    #[test]
    fn set_track_volume_sets_volume() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        b.tracks[0].volume = 0.1;
        b.tracks[1].volume = 0.2;
        let undo = Command::SetTrackVolume {
            track_id: 0,
            volume: 0.3,
        }
        .execute(&mut b);
        assert_eq!(
            undo,
            Command::SetTrackVolume {
                track_id: 0,
                volume: 0.1
            }
        );
        assert_eq!(b.tracks[0].volume, 0.3);
        assert_eq!(b.tracks[1].volume, 0.2);
    }

    #[test]
    fn set_track_volume_on_track_that_does_not_exist_does_nothing() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        let undo = Command::SetTrackVolume {
            track_id: 1000, // Out of range.
            volume: 0.3,
        }
        .execute(&mut b);
        assert_eq!(undo, Command::None);
    }

    #[test]
    fn set_sequence_sets_sequence_on_track() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        b.tracks[4].sequence = vec![MidiEvent {
            position: Position::new(0.0),
            midi: MidiMessage::TuneRequest,
        }];
        let undo = Command::SetSequence {
            track_id: 4,
            sequence: vec![MidiEvent {
                position: Position::new(1.2),
                midi: MidiMessage::Reset,
            }],
        }
        .execute(&mut b);
        assert_eq!(
            undo,
            Command::SetSequence {
                track_id: 4,
                sequence: vec![MidiEvent {
                    position: Position::new(0.0),
                    midi: MidiMessage::TuneRequest,
                },]
            }
        );
        assert_eq!(
            b.tracks[4].sequence,
            vec![MidiEvent {
                position: Position::new(1.2),
                midi: MidiMessage::Reset
            }]
        );
    }

    #[test]
    fn set_record() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        b.recording_enabled = true;

        // true -> true
        let undo = Command::SetRecord(true).execute(&mut b);
        assert_eq!(b.recording_enabled, true);
        assert_eq!(undo, Command::SetRecord(true));

        // true -> false
        let undo = Command::SetRecord(false).execute(&mut b);
        assert_eq!(b.recording_enabled, false);
        assert_eq!(undo, Command::SetRecord(true));

        // false -> false
        let undo = Command::SetRecord(false).execute(&mut b);
        assert_eq!(b.recording_enabled, false);
        assert_eq!(undo, Command::SetRecord(false));

        // false -> true
        let undo = Command::SetRecord(true).execute(&mut b);
        assert_eq!(b.recording_enabled, true);
        assert_eq!(undo, Command::SetRecord(false));
    }
}
