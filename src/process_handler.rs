use jack::PortSpec;
use log::error;
use std::sync::mpsc;

struct RawFn {
    f: Box<dyn Send + FnOnce(&mut ProcessHandler)>,
}

pub struct Mutator {
    sender: mpsc::SyncSender<RawFn>,
}

impl Mutator {
    pub fn mutate(&self, f: impl 'static + Send + FnOnce(&mut ProcessHandler)) {
        let raw_fn = RawFn {
            f: Box::new(move |process_handler| {
                f(process_handler);
            }),
        };
        self.sender.send(raw_fn).unwrap();
    }
}

pub struct ProcessHandler {
    pub plugin_instance: Option<livi::Instance>,
    audio_outputs: [jack::Port<jack::AudioOut>; 2],
    midi_input: jack::Port<jack::MidiIn>,
    atom_sequence_input: livi::event::LV2AtomSequence,
    midi_urid: u32,
    fns_queue: mpsc::Receiver<RawFn>,
}

impl ProcessHandler {
    pub fn new(c: &jack::Client, features: &livi::Features) -> Result<Self, jack::Error> {
        let audio_outputs = [
            c.register_port("out1", jack::AudioOut)?,
            c.register_port("out2", jack::AudioOut)?,
        ];
        let midi_input = c.register_port("midi", jack::MidiIn)?;
        let atom_sequence_input = livi::event::LV2AtomSequence::new(features, 4096);
        let midi_urid = features.midi_urid();
        let (_, fns_queue) = mpsc::sync_channel(1);
        Ok(ProcessHandler {
            plugin_instance: None,
            audio_outputs,
            midi_input,
            atom_sequence_input,
            midi_urid,
            fns_queue,
        })
    }

    pub fn reset_mutator(&mut self) -> Mutator {
        let (tx, rx) = mpsc::sync_channel(16);
        self.fns_queue = rx;
        Mutator { sender: tx }
    }

    pub fn connect(&self, c: &jack::Client) -> Result<(), jack::Error> {
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

    fn handle_fns(&mut self) -> Result<(), mpsc::TryRecvError> {
        let mut f = self.fns_queue.try_recv()?;
        loop {
            (f.f)(self);
            f = self.fns_queue.try_recv()?;
        }
    }
}

impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        let _ = self.handle_fns();
        self.atom_sequence_input.clear();
        for midi in self.midi_input.iter(ps) {
            self.atom_sequence_input
                .push_midi_event::<4>(midi.time as i64, self.midi_urid, midi.bytes)
                .unwrap();
        }
        let ports = livi::EmptyPortConnections::new()
            .with_audio_outputs(self.audio_outputs.iter_mut().map(|p| p.as_mut_slice(ps)))
            .with_atom_sequence_inputs(std::iter::once(&self.atom_sequence_input));

        let res = self
            .plugin_instance
            .as_mut()
            .map(|i| unsafe { i.run(ps.n_frames() as usize, ports) })
            .unwrap_or(Ok(()));
        if let Err(err) = res {
            let p = self.plugin_instance.take().unwrap();
            error!("{:?}", err);
            error!("Disabling plugin {:?}.", p.raw().instance().uri());
        }
        jack::Control::Continue
    }
}
