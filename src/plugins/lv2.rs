//! LV2 plugin integration.
use std::sync::Arc;

use anyhow::anyhow;
use log::*;

use super::plugin_trait::GenericPlugin;

/// Creates instances of `Lv2Plugin`.
pub struct Lv2PluginFactory {
    /// Contains all the known LV2 plugins.
    world: livi::World,
    /// The features to use for LV2.
    features: Arc<livi::Features>,
    /// The sample rate to use for plugin instantiation.
    sample_rate: f64,
}

impl Lv2PluginFactory {
    /// Create a new `Lv2PluginFactory` that supports the given
    /// `sample_rate`.
    pub fn new(sample_rate: f64) -> Lv2PluginFactory {
        let world = livi::World::new();
        let features = world.build_features(livi::FeaturesBuilder {
            min_block_length: 1,
            max_block_length: 8192,
        });
        Lv2PluginFactory {
            world,
            features,
            sample_rate,
        }
    }

    /// Iterate through all the plugins.
    pub fn iter_plugins(&self) -> impl '_ + Iterator<Item = livi::Plugin> {
        self.world.iter_plugins()
    }

    /// Instantiate a plugin by uri. The uri of a plugin can be
    /// obtained by calling `Self::uri()` for an element in
    /// `Self::iter_plugins()`.
    ///
    /// # Safety
    /// Will call foreign likely unsafe code from the plugin.
    pub unsafe fn instantiate(&self, uri: &str) -> anyhow::Result<Lv2Plugin> {
        let plugin = match self.world.iter_plugins().find(|p| p.uri() == uri) {
            Some(p) => p,
            None => return Err(anyhow!("failed to find plugin {}", uri)),
        };
        let plugin_instance = plugin.instantiate(self.features.clone(), self.sample_rate)?;
        Ok(Lv2Plugin {
            error: None,
            plugin_instance,
            midi_urid: self.features.midi_urid(),
            events_input: livi::event::LV2AtomSequence::new(&self.features, 4096),
        })
    }
}

/// An LV2 plugin wrapper.
#[derive(Debug)]
pub struct Lv2Plugin {
    /// The most recent error.
    pub error: Option<livi::error::RunError>,
    /// The plugin instance.
    plugin_instance: livi::Instance,
    /// The URID for midi.
    midi_urid: u32,
    /// The events buffer to pass to `plugin_instance`.
    events_input: livi::event::LV2AtomSequence,
}

impl GenericPlugin for Lv2Plugin {
    /// Process a single chunk of audio.
    fn process<'a>(
        &mut self,
        midi_in: impl Iterator<Item = (u32, &'a [u8])>,
        left_out: &mut [f32],
        right_out: &mut [f32],
    ) {
        if self.error.is_some() {
            return;
        }
        reset_events_from_midi_iter(&mut self.events_input, self.midi_urid, midi_in);
        let samples = left_out.len().min(right_out.len());
        let ports = livi::PortConnections {
            audio_inputs: std::iter::empty(),
            audio_outputs: [left_out, right_out].into_iter(),
            atom_sequence_inputs: std::iter::once(&self.events_input),
            atom_sequence_outputs: std::iter::empty(),
            cv_inputs: std::iter::empty(),
            cv_outputs: std::iter::empty(),
        };
        if let Err(err) = unsafe { self.plugin_instance.run(samples, ports) } {
            warn!("Encountered plugin error: {:?}", err);
            self.error = Some(err);
        }
    }
}

/// Resets all the events in `events` and adds the midi messages from
/// `midi_in`.
fn reset_events_from_midi_iter<'a>(
    events: &mut livi::event::LV2AtomSequence,
    midi_urid: u32,
    midi_in: impl Iterator<Item = (u32, &'a [u8])>,
) {
    events.clear();
    for (frame, msg) in midi_in {
        if let Err(err) = events.push_midi_event::<4>(frame as i64, midi_urid, msg) {
            warn!("Dropping midi message: {}", err);
        }
    }
}

#[cfg(test)]
mod tests {
    use wmidi::MidiMessage;

    use super::*;

    #[test]
    fn test_lv2_plugin_factory_instantiate_on_inexistant_plugin_returns_error() {
        let factory = Lv2PluginFactory::new(44100.0);
        assert!(unsafe { factory.instantiate("").is_err() });
    }

    #[test]
    fn test_lv2_plugin_factory_can_instantiate_valid_plugin() {
        let factory = Lv2PluginFactory::new(44100.0);
        unsafe { factory.instantiate("http://drobilla.net/plugins/mda/EPiano") }.unwrap();
    }

    #[test]
    fn test_lv2_plugin_factory_has_plugins() {
        let factory = Lv2PluginFactory::new(44100.0);
        assert_ne!(factory.iter_plugins().count(), 0)
    }

    #[test]
    fn test_lv2_mismatch_channel_lengths_on_processes_the_smallest_length() {
        let factory = Lv2PluginFactory::new(44100.0);
        let mut plugin =
            unsafe { factory.instantiate("http://drobilla.net/plugins/mda/EPiano") }.unwrap();
        let mut left = [0.0];
        let mut right = [0.0, 0.0];
        let note_on = MidiMessage::NoteOn(
            wmidi::Channel::Ch1,
            wmidi::Note::C4,
            wmidi::U7::from_u8_lossy(100),
        )
        .to_vec();
        plugin.process(
            std::iter::once((0, note_on.as_slice())),
            &mut left,
            &mut right,
        );

        // First frame for both.
        assert_ne!(left, [0.0]);
        assert_ne!(right[0], 0.0);
        // Second frame.
        assert_eq!(right[1], 0.0);
    }

    #[test]
    fn test_lv2_process_with_no_press_is_silent() {
        let factory = Lv2PluginFactory::new(44100.0);
        let mut plugin =
            unsafe { factory.instantiate("http://drobilla.net/plugins/mda/EPiano") }.unwrap();
        let (left, right) = plugin.process_to_vec(2, std::iter::empty());
        assert_eq!(left, vec![0.0, 0.0]);
        assert_eq!(right, vec![0.0, 0.0]);
    }

    #[test]
    fn test_lv2_process_with_note_on_produces_audio() {
        let factory = Lv2PluginFactory::new(44100.0);
        let mut plugin =
            unsafe { factory.instantiate("http://drobilla.net/plugins/mda/EPiano") }.unwrap();
        let note_on = MidiMessage::NoteOn(
            wmidi::Channel::Ch1,
            wmidi::Note::C4,
            wmidi::U7::from_u8_lossy(100),
        )
        .to_vec();
        let (left, right) = plugin.process_to_vec(2, std::iter::once((0, note_on.as_slice())));
        assert_ne!(left, vec![0.0, 0.0]);
        assert_ne!(right, vec![0.0, 0.0]);
    }

    #[test]
    fn test_lv2_process_with_good_plugin_does_not_produce_error() {
        let factory = Lv2PluginFactory::new(44100.0);
        let mut plugin =
            unsafe { factory.instantiate("http://drobilla.net/plugins/mda/EPiano") }.unwrap();
        plugin.process_to_vec(2, std::iter::empty());
        assert!(plugin.error.is_none())
    }

    #[test]
    fn test_lv2_process_with_bad_plugin_saves_error() {
        let factory = Lv2PluginFactory::new(44100.0);
        // This plugin's IO configuration is not supported.
        let mut plugin =
            unsafe { factory.instantiate("http://drobilla.net/plugins/mda/TalkBox") }.unwrap();

        assert!(plugin.error.is_none());
        plugin.process_to_vec(2, std::iter::empty());
        assert!(plugin.error.is_some());
    }
}
