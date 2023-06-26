use jack::PortSpec;
use log::error;
use std::sync::mpsc;

struct RawFn {
    f: Box<dyn Send + FnOnce(&mut ProcessHandler)>,
}

pub struct RemoteExecutor {
    sender: mpsc::SyncSender<RawFn>,
}

impl RemoteExecutor {
    fn base_call(&self, f: impl 'static + Send + FnOnce(&mut ProcessHandler)) {
        let raw_fn = RawFn {
            f: Box::new(move |process_handler| {
                f(process_handler);
            }),
        };
        self.sender.send(raw_fn).unwrap();
    }

    pub fn execute<T: 'static + Send>(
        &self,
        f: impl 'static + Send + FnOnce(&mut ProcessHandler) -> T,
    ) -> T {
        let (tx, rx) = mpsc::sync_channel(1);
        self.base_call(move |ps| {
            let ret = f(ps);
            tx.send(ret).unwrap();
        });
        rx.recv().unwrap()
    }
}

pub struct ProcessHandler {
    pub plugin_instance: Option<livi::Instance>,
    audio_outputs: [jack::Port<jack::AudioOut>; 2],
    midi_input: jack::Port<jack::MidiIn>,
    atom_sequence_input: livi::event::LV2AtomSequence,
    midi_urid: u32,
    remote_fns: mpsc::Receiver<RawFn>,
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
        let (_, remote_fns) = mpsc::sync_channel(1);
        Ok(ProcessHandler {
            plugin_instance: None,
            audio_outputs,
            midi_input,
            atom_sequence_input,
            midi_urid,
            remote_fns,
        })
    }

    pub fn reset_remote_executor(&mut self, queue_size: usize) -> RemoteExecutor {
        let (tx, rx) = mpsc::sync_channel(queue_size);
        self.remote_fns = rx;
        RemoteExecutor { sender: tx }
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

    fn handle_remote_fns(&mut self) -> Result<(), mpsc::TryRecvError> {
        let mut f = self.remote_fns.try_recv()?;
        loop {
            (f.f)(self);
            f = self.remote_fns.try_recv()?;
        }
    }
}

impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        match self.handle_remote_fns() {
            // All the scenarios are ok.
            Ok(_) | Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => (),
        };
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
            // TODO: Drop this outside of processing thread.
            let p = self.plugin_instance.take().unwrap();
            error!("{:?}", err);
            error!("Disabling plugin {:?}.", p.raw().instance().uri());
        }
        jack::Control::Continue
    }
}
