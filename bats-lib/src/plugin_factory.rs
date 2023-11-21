use bats_dsp::sample_rate::SampleRate;
use serde::{Deserialize, Serialize};

use crate::plugin::{empty::Empty, toof::Toof, BatsInstrument};

/// An object that is used to build plugins.
#[derive(Copy, Clone, Default, Debug)]
pub enum PluginBuilder {
    /// An empty plugin that does nothing.
    #[default]
    Empty,
    /// The toof plugin.
    Toof,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AnyPlugin {
    Empty(Empty),
    Toof(Box<Toof>),
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
    pub fn plugin(&'_ self) -> &'_ dyn BatsInstrument {
        match self {
            AnyPlugin::Empty(p) => p,
            AnyPlugin::Toof(p) => p.as_ref(),
        }
    }

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
