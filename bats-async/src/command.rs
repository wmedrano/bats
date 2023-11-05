use bats_lib::{plugin::BatsInstrument, Bats, Track};
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
    AddTrack(Track),
    /// Remove a track.
    RemoveTrack { id: u32 },
    /// Set the armed track or use `None` to not arm any track.
    SetArmedTrack(Option<u32>),
    /// Set the track volume.
    SetTrackVolume { track_id: u32, volume: f32 },
    /// Set a parameter.
    SetParam {
        track_id: u32,
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
            Command::AddTrack(track) => {
                let id = track.id;
                b.tracks.push(track);
                Command::RemoveTrack { id }
            }
            Command::RemoveTrack { id } => match b.tracks.iter().position(|p| p.id == id) {
                None => Command::None,
                Some(idx) => Command::AddTrack(b.tracks.remove(idx)),
            },
            Command::SetTrackVolume {
                track_id: track,
                volume,
            } => match b.tracks.iter_mut().find(|t| t.id == track) {
                None => Command::None,
                Some(t) => {
                    let undo = Command::SetTrackVolume {
                        track_id: track,
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
            } => match b.tracks.iter_mut().find(|t| t.id == track_id) {
                Some(t) => {
                    let undo = Command::SetParam {
                        track_id,
                        param_id,
                        value: t.plugin.param(param_id),
                    };
                    t.plugin.set_param(param_id, value);
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
        }
    }
}

#[cfg(test)]
mod tests {
    use bats_dsp::{buffers::Buffers, sample_rate::SampleRate};
    use bats_lib::plugin::toof::Toof;

    use super::*;

    fn get_track_volumes(b: &Bats) -> Vec<f32> {
        b.tracks.iter().map(|t| t.volume).collect()
    }

    fn get_track_ids(b: &Bats) -> Vec<u32> {
        b.tracks.iter().map(|t| t.id).collect()
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
    fn add_track() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        let intial_track = Track {
            id: 0,
            plugin: Toof::new(b.sample_rate),
            volume: 1.0,
            output: Buffers::new(64),
        };
        b.tracks.push(intial_track.clone());
        assert_eq!(b.tracks.len(), 1);

        let new_track = Track {
            id: 10,
            plugin: Toof::new(b.sample_rate),
            volume: 1.0,
            output: Buffers::new(64),
        };
        let undo = Command::AddTrack(new_track.clone()).execute(&mut b);
        assert_eq!(b.tracks, vec![intial_track.clone(), new_track.clone()]);
        assert_eq!(undo, Command::RemoveTrack { id: 10 });

        let undo = Command::RemoveTrack { id: 10 }.execute(&mut b);
        assert_eq!(b.tracks, vec![intial_track]);
        assert_eq!(undo, Command::AddTrack(new_track.clone()));
    }

    #[test]
    fn remove_track_that_does_not_exist_does_nothing() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        b.tracks.push(Track {
            id: 0,
            plugin: Toof::new(b.sample_rate),
            volume: 1.0,
            output: Buffers::new(64),
        });
        b.tracks.push(Track {
            id: 1,
            plugin: Toof::new(b.sample_rate),
            volume: 1.0,
            output: Buffers::new(64),
        });
        assert_eq!(get_track_ids(&b), vec![0, 1]);
        assert_eq!(
            Command::RemoveTrack { id: 100 }.execute(&mut b),
            Command::None
        );
        assert_eq!(get_track_ids(&b), vec![0, 1]);
    }

    #[test]
    fn set_armed_track() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        b.armed_track = None;

        let undo = Command::SetArmedTrack(Some(10)).execute(&mut b);
        assert_eq!(b.armed_track, Some(10));
        assert_eq!(undo, Command::SetArmedTrack(None));

        let undo = Command::SetArmedTrack(Some(20)).execute(&mut b);
        assert_eq!(b.armed_track, Some(20));
        assert_eq!(undo, Command::SetArmedTrack(Some(10)));

        let undo = Command::SetArmedTrack(None).execute(&mut b);
        assert_eq!(b.armed_track, None);
        assert_eq!(undo, Command::SetArmedTrack(Some(20)));
    }

    #[test]
    fn set_track_volume_sets_volume() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        b.tracks.push(Track {
            id: 10,
            plugin: Toof::new(b.sample_rate),
            volume: 0.1,
            output: Buffers::new(64),
        });
        b.tracks.push(Track {
            id: 20,
            plugin: Toof::new(b.sample_rate),
            volume: 0.2,
            output: Buffers::new(64),
        });
        assert_eq!(get_track_volumes(&b), vec![0.1, 0.2]);
        let undo = Command::SetTrackVolume {
            track_id: 20,
            volume: 0.3,
        }
        .execute(&mut b);
        assert_eq!(
            undo,
            Command::SetTrackVolume {
                track_id: 20,
                volume: 0.2
            }
        );
        assert_eq!(get_track_volumes(&b), vec![0.1, 0.3]);
    }

    #[test]
    fn set_track_volume_on_track_that_does_not_exist_does_nothing() {
        let mut b = Bats::new(SampleRate::new(44100.0), 64);
        b.tracks.push(Track {
            id: 10,
            plugin: Toof::new(b.sample_rate),
            volume: 0.1,
            output: Buffers::new(64),
        });
        assert_eq!(get_track_volumes(&b), vec![0.1]);
        let undo = Command::SetTrackVolume {
            track_id: 20,
            volume: 0.3,
        }
        .execute(&mut b);
        assert_eq!(undo, Command::None);
        assert_eq!(get_track_volumes(&b), vec![0.1]);
    }
}
