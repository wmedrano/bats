use anyhow::Result;

use crate::processor::Processor;

/// `JackAdapter` implements the real-time audio component of bats.
pub struct JackAdapter {
    /// The processor.
    processor: Processor,
    /// The midi input.
    midi_in: jack::Port<jack::MidiIn>,
    /// The left audio output.
    out_left: jack::Port<jack::AudioOut>,
    /// The right audio output.
    out_right: jack::Port<jack::AudioOut>,
}

impl JackAdapter {
    /// Create a new `JackAdapter`.
    pub fn new(client: &jack::Client) -> Result<JackAdapter> {
        Ok(JackAdapter {
            processor: Processor::default(),
            midi_in: client.register_port("midi_in", jack::MidiIn)?,
            out_left: client.register_port("out_left", jack::AudioOut)?,
            out_right: client.register_port("out_right", jack::AudioOut)?,
        })
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
