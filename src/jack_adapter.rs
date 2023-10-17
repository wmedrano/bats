use anyhow::Result;

pub struct ProcessHandler {
    ports: Ports,
}

impl ProcessHandler {
    pub fn new(ports: Ports) -> ProcessHandler {
        ProcessHandler { ports }
    }
}

impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        clear(self.ports.left.as_mut_slice(ps));
        clear(self.ports.right.as_mut_slice(ps));
        for _ in self.ports.midi.iter(ps) {}
        jack::Control::Continue
    }
}

pub struct Ports {
    left: jack::Port<jack::AudioOut>,
    right: jack::Port<jack::AudioOut>,
    midi: jack::Port<jack::MidiIn>,
}

impl Ports {
    pub fn new(c: &jack::Client) -> Result<Ports> {
        Ok(Ports {
            left: c.register_port("left", jack::AudioOut)?,
            right: c.register_port("right", jack::AudioOut)?,
            midi: c.register_port("midi", jack::MidiIn)?,
        })
    }
}

fn clear(s: &mut [f32]) {
    for v in s.iter_mut() {
        *v = 0.0;
    }
}
