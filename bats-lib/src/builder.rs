use bats_dsp::sample_rate::SampleRate;
use serde::{Deserialize, Serialize};

use crate::plugin::{empty::Empty, toof::Toof, BatsInstrument};
use crate::track::Track;
use crate::transport::Transport;
use crate::Bats;

/// Creates a bats builder.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BatsBuilder {
    /// The sample rate.
    pub sample_rate: SampleRate,
    /// The buffer size.
    pub buffer_size: usize,
    /// The bpm.
    pub bpm: f32,
    /// The builders for the tracks.
    pub tracks: [TrackBuilder; Bats::SUPPORTED_TRACKS],
}

/// Creates a track.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrackBuilder {
    /// The plugin builder.
    pub plugin: PluginBuilder,
    /// The volume for the track.
    pub volume: f32,
}

/// An object that is used to build plugins.
#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub enum PluginBuilder {
    /// An empty plugin that does nothing.
    #[default]
    Empty,
    /// The toof plugin.
    Toof,
}

/// Contains all the plugins.
#[derive(Clone, Debug, PartialEq)]
pub enum AnyPlugin {
    /// The empty plugin.
    Empty(Empty),
    /// The toof plugin.
    Toof(Box<Toof>),
}

impl BatsBuilder {
    /// Build the bats object.
    pub fn build(&self) -> Bats {
        Bats {
            transport: Transport::new(self.sample_rate, self.buffer_size, self.bpm),
            armed_track: 0,
            recording_enabled: false,
            sample_rate: self.sample_rate,
            buffer_size: self.buffer_size,
            midi_buffer: Vec::with_capacity(self.buffer_size * 8),
            tracks: core::array::from_fn(|idx| {
                self.tracks[idx].build(self.sample_rate, self.buffer_size)
            }),
        }
    }

    /// Create a builder from a bats object.
    pub fn from_bats(b: &Bats) -> BatsBuilder {
        BatsBuilder {
            sample_rate: b.sample_rate,
            buffer_size: b.buffer_size,
            bpm: b.transport.bpm(),
            tracks: core::array::from_fn(|idx| {
                let track = &b.tracks[idx];
                TrackBuilder::from_bats(track)
            }),
        }
    }
}

impl TrackBuilder {
    /// Build the track.
    pub fn build(&self, sample_rate: SampleRate, buffer_size: usize) -> Track {
        Track {
            plugin: self.plugin.build(sample_rate),
            volume: self.volume,
            ..Track::new(buffer_size)
        }
    }

    /// Create a track builder from a track.
    pub fn from_bats(t: &Track) -> TrackBuilder {
        TrackBuilder {
            plugin: PluginBuilder::from_bats(&t.plugin),
            volume: t.volume,
        }
    }
}

impl AnyPlugin {
    /// Get a reference to the underlying plugin.
    pub fn plugin(&'_ self) -> &'_ dyn BatsInstrument {
        match self {
            AnyPlugin::Empty(p) => p,
            AnyPlugin::Toof(p) => p.as_ref(),
        }
    }

    /// Get a mutable reference to the underlying plugin.
    pub fn plugin_mut(&'_ mut self) -> &'_ mut dyn BatsInstrument {
        match self {
            AnyPlugin::Empty(p) => p,
            AnyPlugin::Toof(p) => p.as_mut(),
        }
    }
}

impl PluginBuilder {
    /// All the plugin builders available.
    pub const ALL: &'static [PluginBuilder] = &[PluginBuilder::Empty, PluginBuilder::Toof];

    /// The name of the plugin.
    pub fn name(self) -> &'static str {
        match self {
            PluginBuilder::Empty => "empty",
            PluginBuilder::Toof => "toof",
        }
    }

    /// Build the new plugin.
    pub fn build(self, sample_rate: SampleRate) -> AnyPlugin {
        match self {
            PluginBuilder::Empty => AnyPlugin::Empty(Empty),
            PluginBuilder::Toof => AnyPlugin::Toof(Toof::new(sample_rate)),
        }
    }

    /// Create a plugin builder from an existing plugin.
    pub fn from_bats(p: &AnyPlugin) -> PluginBuilder {
        match p {
            AnyPlugin::Empty(_) => PluginBuilder::Empty,
            AnyPlugin::Toof(_) => PluginBuilder::Toof,
        }
    }
}

impl From<Box<Toof>> for AnyPlugin {
    fn from(v: Box<Toof>) -> AnyPlugin {
        AnyPlugin::Toof(v)
    }
}

impl Default for AnyPlugin {
    fn default() -> AnyPlugin {
        AnyPlugin::Empty(Empty)
    }
}

impl Default for TrackBuilder {
    fn default() -> TrackBuilder {
        TrackBuilder {
            plugin: PluginBuilder::default(),
            volume: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build() {
        let initial_bats = {
            let mut b = BatsBuilder {
                sample_rate: SampleRate::new(48000.0),
                buffer_size: 256,
                bpm: 175.2,
                tracks: Default::default(),
            }
            .build();
            b.tracks[1].volume = 0.65;
            b.tracks[1].plugin = Toof::new(b.sample_rate).into();
            b
        };
        let initial_builder = BatsBuilder::from_bats(&initial_bats);
        let new_bats = initial_builder.build();
        let new_builder = BatsBuilder::from_bats(&new_bats);
        assert_eq!(initial_bats, new_bats);
        assert_eq!(initial_builder, new_builder);
    }
}
