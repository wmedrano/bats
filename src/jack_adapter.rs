//! Hook up bats! to JACK for audio and midi IO.
use anyhow::Result;
use jack::PortSpec;
use log::*;

use crate::processor::{Processor, ProcessorCommunicator};

/// `JackAdapter` implements the real-time audio component of bats.
#[derive(Debug)]
pub struct JackAdapter {
    /// The processor.
    pub processor: Processor,
    /// The midi input.
    midi_in: jack::Port<jack::MidiIn>,
    /// The left audio output.
    out_left: jack::Port<jack::AudioOut>,
    /// The right audio output.
    out_right: jack::Port<jack::AudioOut>,
}

impl JackAdapter {
    /// Create a new `JackAdapter`.
    pub fn new(client: &jack::Client) -> Result<(JackAdapter, ProcessorCommunicator)> {
        let (processor, processor_tx) = Processor::new(client.buffer_size() as usize * 2);
        let midi_in = client.register_port("midi_in", jack::MidiIn)?;
        let out_left = client.register_port("out_left", jack::AudioOut)?;
        let out_right = client.register_port("out_right", jack::AudioOut)?;

        Ok((
            JackAdapter {
                processor,
                midi_in,
                out_left,
                out_right,
            },
            processor_tx,
        ))
    }

    /// Returns a function that connects the ports in `self` to
    /// physical ports. The connector function has a higher chance of
    /// suceeding if called after `self` has been activated for
    /// processing.
    pub fn connect_ports_fn(&self) -> impl FnOnce() -> Result<()> {
        let self_midi_in_port = self.midi_in.name();
        let self_audio_ports = [self.out_left.name(), self.out_right.name()];

        move || {
            let (client, _) =
                jack::Client::new("bats_connector", jack::ClientOptions::NO_START_SERVER)?;
            // Connect physical midi ports.
            let physical_midi_in_ports = client.ports(
                None,
                Some(jack::MidiOut.jack_port_type()),
                jack::PortFlags::IS_PHYSICAL | jack::PortFlags::IS_OUTPUT,
            );
            let dst = self_midi_in_port?;
            for src in physical_midi_in_ports {
                info!("Connecting midi port {} to {}.", src, dst);
                client.connect_ports_by_name(&src, &dst)?;
            }
            // Connect physical audio ports.
            let physical_audio_out_ports = client.ports(
                None,
                Some(jack::AudioIn.jack_port_type()),
                jack::PortFlags::IS_PHYSICAL | jack::PortFlags::IS_INPUT,
            );
            for (src, dst) in self_audio_ports.into_iter().zip(physical_audio_out_ports) {
                let src = src?;
                info!("Connecting audio port {} to {}.", src, dst);
                client.connect_ports_by_name(&src, &dst)?;
            }
            Ok(())
        }
    }
}

impl jack::ProcessHandler for JackAdapter {
    /// Handle processing for JACK.
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        self.processor.process(
            self.midi_in.iter(ps).map(|r| (r.time, r.bytes)),
            self.out_left.as_mut_slice(ps),
            self.out_right.as_mut_slice(ps),
        );
        jack::Control::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_new_is_ok() {
        let (c, _) = jack::Client::new(
            "test_processor_new_is_ok",
            jack::ClientOptions::NO_START_SERVER,
        )
        .unwrap();
        JackAdapter::new(&c).unwrap();
    }
}
