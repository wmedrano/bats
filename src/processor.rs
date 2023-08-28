use anyhow::Result;

/// Processor implements the real-time audio component of bats.
pub struct Processor {
    /// The midi input.
    midi_in: jack::Port<jack::MidiIn>,
    /// The left audio output.
    out_left: jack::Port<jack::AudioOut>,
    /// The right audio output.
    out_right: jack::Port<jack::AudioOut>,
}

impl Processor {
    /// Create a new processor.
    pub fn new(client: &jack::Client) -> Result<Processor> {
        let midi_in = client.register_port("midi_in", jack::MidiIn)?;
        let out_left = client.register_port("out_left", jack::AudioOut)?;
        let out_right = client.register_port("out_right", jack::AudioOut)?;
        Ok(Processor {
            midi_in,
            out_left,
            out_right,
        })
    }
}

impl jack::ProcessHandler for Processor {
    /// Handle processing for JACK.
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        for _ in self.midi_in.iter(ps) {}
        clear(self.out_left.as_mut_slice(ps));
        clear(self.out_right.as_mut_slice(ps));
        jack::Control::Continue
    }
}

/// Assign all values in `slice` to `0.0`.
fn clear(slice: &mut [f32]) {
    for v in slice.iter_mut() {
        *v = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_new_is_ok() {
        let (c, _) =
            jack::Client::new("test_processor_new_is_ok", jack::ClientOptions::empty()).unwrap();
        Processor::new(&c).unwrap();
    }
}
