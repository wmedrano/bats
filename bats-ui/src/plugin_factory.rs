use bats_dsp::sample_rate::SampleRate;
use bats_lib::plugin::toof::Toof;

/// An object that is used to build plugins.
#[derive(Copy, Clone, Default, Debug)]
pub enum PluginBuilder {
    /// An empty plugin that does nothing.
    #[default]
    Empty,
    /// The toof plugin.
    Toof,
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
    pub fn build(self, sample_rate: SampleRate) -> Option<Box<Toof>> {
        match self {
            PluginBuilder::Empty => None,
            PluginBuilder::Toof => Some(Toof::new(sample_rate)),
        }
    }
}
