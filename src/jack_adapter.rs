use anyhow::Result;

use crate::bats::Bats;

/// Implements the JACK processor.
#[derive(Debug)]
pub struct ProcessHandler {
    /// The IO ports.
    ports: Ports,
    /// The bats processing object.
    bats: Bats,
    /// An intermediate midi buffer.
    midi_buffer: Vec<(u32, wmidi::MidiMessage<'static>)>,
}

impl ProcessHandler {
    /// Create a new `ProcessHandler` with ports registered from `c`.
    pub fn new(c: &jack::Client) -> Result<ProcessHandler> {
        Ok(ProcessHandler {
            ports: Ports::new(c)?,
            bats: Bats::default(),
            midi_buffer: Vec::with_capacity(4096),
        })
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
        self.bats.process(
            self.midi_buffer.iter(),
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
}
