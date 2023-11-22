use bats_dsp::sample_rate::SampleRate;
use serde::{Deserialize, Serialize};

use crate::plugin::{empty::Empty, toof::Toof, BatsInstrument};
use crate::track::Track;
use crate::Bats;
use crate::transport::Transport;

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
#[derive(Copy, Clone, Default,Debug, Serialize, Deserialize, PartialEq)]
pub struct TrackBuilder {
    /// The plugin builder.
    pub plugin: PluginBuilder,
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
}

impl TrackBuilder {
    /// Build the track.
    pub fn build(&self, sample_rate: SampleRate, buffer_size: usize) -> Track {
        let mut t = Track::new(buffer_size);
        t.plugin = self.plugin.build(sample_rate);
        t
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
}
