use bmidi::MidiMessage;
use serde::{Deserialize, Serialize};

use super::{metadata::Metadata, BatsInstrument};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Empty;

impl BatsInstrument for Empty {
    fn metadata(&self) -> &'static Metadata {
        &Metadata {
            name: "empty",
            params: &[],
        }
    }

    fn handle_midi(&mut self, _: &MidiMessage) {}

    fn process(&mut self) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn param(&self, _: u32) -> f32 {
        0.0
    }

    fn set_param(&mut self, _: u32, _: f32) {}

    fn batch_cleanup(&mut self) {}
}
