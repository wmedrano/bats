use bats_lib::{
    plugin::toof::Toof,
    plugin::{BatsInstrument, MidiEvent},
    Bats,
};
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
    SetPlugin {
        track_id: usize,
        plugin: Option<Box<Toof>>,
    },
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
                    if plugin.is_none() && t.plugin.is_none() {
                        return Command::None;
                    }
                    let undo = Command::SetPlugin {
                        track_id,
                        plugin: t.plugin.take(),
                    };
                    t.plugin = plugin;
                    undo
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
                Some(t) => match t.plugin.as_mut() {
                    None => Command::None,
                    Some(p) => {
                        let undo = Command::SetParam {
                            track_id,
                            param_id,
                            value: p.param(param_id),
                        };
                        p.set_param(param_id, value);
                        undo
                    }
                },
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
        }
    }
}

#[cfg(test)]
mod tests {
    use bats_dsp::{position::Position, sample_rate::SampleRate};
    use bats_lib::plugin::toof::Toof;
    use wmidi::MidiMessage;

    use super::*;

    /// Get all the track name for non-empty tracks.
    fn get_track_names(b: &Bats) -> Vec<&'static str> {
        b.tracks
            .iter()
            .flat_map(|t| t.plugin.as_ref().map(|p| p.metadata().name))
            .collect()
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
        let plugin = Some(Toof::new(b.sample_rate));
        b.tracks[0].plugin = plugin.clone();
        assert_eq!(get_track_names(&b), vec!["toof"]);

        let undo = Command::SetPlugin {
            track_id: 1,
            plugin: plugin.clone(),
        }
        .execute(&mut b);
        assert_eq!(get_track_names(&b), vec!["toof", "toof"]);
        assert_eq!(
            undo,
            Command::SetPlugin {
                track_id: 1,
                plugin: None
            }
        );

        let undo = Command::SetPlugin {
            track_id: 1,
            plugin: None,
        }
        .execute(&mut b);
        assert_eq!(get_track_names(&b), vec!["toof"]);
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
        let plugin = Some(Toof::new(b.sample_rate));
        b.tracks[0].plugin = plugin.clone();
        b.tracks[2].plugin = plugin.clone();
        assert_eq!(get_track_names(&b), vec!["toof", "toof"]);
        assert_eq!(
            Command::SetPlugin {
                track_id: 1,
                plugin: None
            }
            .execute(&mut b),
            Command::None
        );
        assert_eq!(get_track_names(&b), vec!["toof", "toof"]);
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
}
