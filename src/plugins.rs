//! Contains integration for plugin processing.
use self::{lv2::Lv2Plugin, plugin_trait::GenericPlugin, sampler::OneShotSampler};

pub mod lv2;
pub mod plugin_trait;
pub mod sampler;

/// A simple wrapper over plugins. If the plugin returns an error
/// during processing, it is disabled by changing to `Plugin::Silent`.
#[derive(Default, Debug)]
pub enum Plugin {
    /// A plugin that produces no output.
    #[default]
    Silent,
    /// A sampler that plays audio on each press.
    OneShotSampler(OneShotSampler),
    /// An LV2 plugin.
    Lv2Plugin(Lv2Plugin),
}

impl From<OneShotSampler> for Plugin {
    fn from(value: OneShotSampler) -> Plugin {
        Plugin::OneShotSampler(value)
    }
}

impl From<Lv2Plugin> for Plugin {
    fn from(value: Lv2Plugin) -> Plugin {
        Plugin::Lv2Plugin(value)
    }
}

impl GenericPlugin for Plugin {
    fn process<'a>(
        &mut self,
        midi_in: impl Iterator<Item = (u32, &'a [u8])>,
        left_out: &mut [f32],
        right_out: &mut [f32],
    ) {
        match self {
            Plugin::Silent => (),
            Plugin::OneShotSampler(s) => s.process(midi_in, left_out, right_out),
            Plugin::Lv2Plugin(p) => p.process(midi_in, left_out, right_out),
        }
    }
}
