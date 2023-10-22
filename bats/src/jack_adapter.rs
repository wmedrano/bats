use anyhow::Result;
use bats_async::{new_async_commander, CommandReceiver, CommandSender};
use bats_lib::Bats;
use jack::PortSpec;
use log::{info, warn};

/// Implements the JACK processor.
#[derive(Debug)]
pub struct ProcessHandler {
    /// The IO ports.
    ports: Ports,
    /// The bats processing object.
    bats: Bats,
    /// Command queue for the bats processing object.
    commands: CommandReceiver,
    /// An intermediate midi buffer.
    midi_buffer: Vec<(u32, wmidi::MidiMessage<'static>)>,
}

impl ProcessHandler {
    /// Create a new `ProcessHandler` with ports registered from `c`.
    pub fn new(c: &jack::Client, bats: Bats) -> Result<(ProcessHandler, CommandSender)> {
        let (command_sender, command_receiver) = new_async_commander();
        Ok((
            ProcessHandler {
                ports: Ports::new(c)?,
                bats,
                commands: command_receiver,
                midi_buffer: Vec::with_capacity(4096),
            },
            command_sender,
        ))
    }

    /// Returns a function that connects this `ProcessHandler`'s
    /// virtual ports to physical ports.
    pub fn connector(&self) -> Result<Box<dyn Send + FnMut()>> {
        let (connector_client, status) =
            jack::Client::new("bats_connector", jack::ClientOptions::NO_START_SERVER)?;
        info!(
            "Created connector client {:?} with status {:?}",
            connector_client, status
        );
        let virtual_ports = self.ports.port_names()?;
        Ok(Box::new(move || {
            let physical_audio_outs = connector_client.ports(
                None,
                Some(jack::AudioIn.jack_port_type()),
                jack::PortFlags::IS_TERMINAL | jack::PortFlags::IS_INPUT,
            );
            for (i, o) in virtual_ports
                .audio_outputs
                .iter()
                .zip(physical_audio_outs.iter())
            {
                let p = connector_client.port_by_name(i.as_str()).unwrap();
                if p.is_connected_to(o.as_str()).unwrap_or(false) {
                    continue;
                }
                info!("Connecting audio port {} to {}.", i, o);
                if let Err(err) = connector_client.connect_ports_by_name(i.as_str(), o.as_str()) {
                    warn!("Failed to connect audio output: {}", err);
                }
            }
            let physical_midi_in = connector_client.ports(
                None,
                Some(jack::MidiOut.jack_port_type()),
                jack::PortFlags::IS_TERMINAL | jack::PortFlags::IS_OUTPUT,
            );
            for i in physical_midi_in {
                let p = connector_client
                    .port_by_name(&virtual_ports.midi_input)
                    .unwrap();
                if p.is_connected_to(&i).unwrap_or(false) {
                    continue;
                }
                info!(
                    "Connecting midi port {} to {}.",
                    i, virtual_ports.midi_input
                );
                if let Err(err) =
                    connector_client.connect_ports_by_name(&i, &virtual_ports.midi_input)
                {
                    warn!("Failed to connect midi input: {}", err);
                }
            }
        }))
    }
}

impl jack::ProcessHandler for ProcessHandler {
    /// Process inputs and fill outputs.
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        self.midi_buffer.clear();
        for m in self.ports.midi.iter(ps) {
            if let Ok(msg) = wmidi::MidiMessage::from_bytes(m.bytes) {
                if let Some(msg) = msg.drop_unowned_sysex() {
                    self.midi_buffer.push((m.time, msg));
                }
            }
        }
        self.commands.execute_all(&mut self.bats);
        self.bats.process(
            self.midi_buffer.as_slice(),
            self.ports.left.as_mut_slice(ps),
            self.ports.right.as_mut_slice(ps),
        );
        jack::Control::Continue
    }
}

/// Contains all the IO ports.
#[derive(Debug)]
pub struct Ports {
    /// The left audio output buffer.
    left: jack::Port<jack::AudioOut>,
    /// The right audio output buffer.
    right: jack::Port<jack::AudioOut>,
    /// The midi input.
    midi: jack::Port<jack::MidiIn>,
}

impl Ports {
    /// Create a new `Ports` object with ports from `c`.
    pub fn new(c: &jack::Client) -> Result<Ports> {
        Ok(Ports {
            left: c.register_port("left", jack::AudioOut)?,
            right: c.register_port("right", jack::AudioOut)?,
            midi: c.register_port("midi", jack::MidiIn)?,
        })
    }

    /// Get all the port names.
    pub fn port_names(&self) -> Result<PortNames> {
        Ok(PortNames {
            audio_outputs: [self.left.name()?, self.right.name()?],
            midi_input: self.midi.name()?,
        })
    }
}

/// Holds all the ports by name.
#[derive(Debug)]
pub struct PortNames {
    /// The audio output ports.
    pub audio_outputs: [String; 2],
    /// The midi input port.
    pub midi_input: String,
}
