use bats_lib::{plugin::toof::Toof, plugin::BatsInstrument, Bats};
use log::error;

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
        }
    }
}

#[cfg(test)]
mod tests {
    use bats_dsp::sample_rate::SampleRate;
    use bats_lib::plugin::toof::Toof;

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
}
