use jack::PortSpec;

use crate::bats::Bats;

/// Handles processing for JACK.
pub struct JackProcessHandler {
    /// The plugin instance to run or `None` if no plugin should be running.
    pub bats: Bats,

    /// The JACK audio ports to output to.
    audio_outputs: [jack::Port<jack::AudioOut>; 2],
    /// The JACK midi port to read midi from.
    midi_input: jack::Port<jack::MidiIn>,
}

impl JackProcessHandler {
    /// Create a new `ProcessHandler`.
    pub fn new(c: &jack::Client, features: &livi::Features) -> Result<Self, jack::Error> {
        let audio_outputs = [
            c.register_port("out1", jack::AudioOut)?,
            c.register_port("out2", jack::AudioOut)?,
        ];
        let midi_input = c.register_port("midi", jack::MidiIn)?;
        let bats = Bats::new(features);
        Ok(JackProcessHandler {
            bats,
            audio_outputs,
            midi_input,
        })
    }

    /// Connects the ports in `self` to physical ports.
    pub fn connect_ports(&self, c: &jack::Client) -> Result<(), jack::Error> {
        // Audio
        let inputs = self.audio_outputs.iter();
        let outputs = c.ports(
            None,
            Some(jack::AudioIn.jack_port_type()),
            jack::PortFlags::IS_PHYSICAL | jack::PortFlags::IS_INPUT,
        );
        for (input, output) in inputs.zip(outputs.iter()) {
            c.connect_ports_by_name(&input.name()?, output)?;
        }

        // Midi
        for input in c.ports(
            None,
            Some(jack::MidiOut.jack_port_type()),
            jack::PortFlags::IS_TERMINAL | jack::PortFlags::IS_OUTPUT,
        ) {
            c.connect_ports_by_name(&input, &self.midi_input.name()?)?;
        }
        Ok(())
    }
}

impl jack::ProcessHandler for JackProcessHandler {
    /// Process data for JACK.
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        self.bats.process(
            ps.n_frames() as usize,
            self.midi_input.iter(ps).map(|m| (m.time as i64, m.bytes)),
            match &mut self.audio_outputs {
                [a, b] => [a.as_mut_slice(ps), b.as_mut_slice(ps)],
            },
        );
        jack::Control::Continue
    }
}
