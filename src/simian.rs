use log::error;

/// Contains a callable function.
struct RawFn {
    /// A callable function.
    f: Box<dyn Send + FnOnce(&mut Simian)>,
}

/// A struct that can execute code on an object that is running on a different thread.
pub struct RemoteExecutor {
    /// The channel to send functions to execute on.
    sender: crossbeam_channel::Sender<RawFn>,
}

impl RemoteExecutor {
    /// Execute `f`. Note that there is no return value and it does not wait for `f` to actually be
    /// executed.
    fn base_call(&self, f: impl 'static + Send + FnOnce(&mut Simian)) {
        let raw_fn = RawFn {
            f: Box::new(move |s| {
                f(s);
            }),
        };
        self.sender.send(raw_fn).unwrap();
    }

    /// Execute `f` and return its value once it has executed. This function will block until the
    /// remote object has received and executed `f`.
    pub fn execute<T: 'static + Send>(
        &self,
        f: impl 'static + Send + FnOnce(&mut Simian) -> T,
    ) -> Result<T, crossbeam_channel::RecvError> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        self.base_call(move |s| {
            let ret = f(s);
            tx.send(ret).unwrap();
        });
        rx.recv()
    }
}

/// Handles audio processing.
pub struct Simian {
    /// The plugin instance to run or `None` if no plugin should be running.
    pub plugin_instance: Option<livi::Instance>,

    /// A temporary `LV2AtomSequence` to use for processing. The object is persisted to avoid
    /// allocating memory.
    atom_sequence_input: livi::event::LV2AtomSequence,
    /// The `urid` for the LV2 midi atom.
    midi_urid: u32,
    /// A channel to receive functions to execute.
    remote_fns: crossbeam_channel::Receiver<RawFn>,
}

impl Simian {
    /// Create a new `ProcessHandler`.
    pub fn new(features: &livi::Features) -> Self {
        let atom_sequence_input = livi::event::LV2AtomSequence::new(features, 4096);
        let midi_urid = features.midi_urid();
        let (_, remote_fns) = crossbeam_channel::bounded(1);
        Simian {
            plugin_instance: None,
            atom_sequence_input,
            midi_urid,
            remote_fns,
        }
    }

    /// Reset the remote executor and return it.
    ///
    /// Any previously set executor will no longer be responsive.
    pub fn reset_remote_executor(&mut self, queue_size: usize) -> RemoteExecutor {
        let (tx, rx) = crossbeam_channel::bounded(queue_size);
        self.remote_fns = rx;
        RemoteExecutor { sender: tx }
    }

    /// Run all remote functions that have been queued.
    fn handle_remote_fns(&mut self) -> Result<(), crossbeam_channel::TryRecvError> {
        let mut f = self.remote_fns.try_recv()?;
        loop {
            (f.f)(self);
            f = self.remote_fns.try_recv()?;
        }
    }

    /// Process data and write the results to `audio_out`.
    pub fn process<'a>(
        &'a mut self,
        frames: usize,
        midi_in: impl Iterator<Item = (i64, &'a [u8])>,
        audio_out: impl ExactSizeIterator + Iterator<Item = &'a mut [f32]>,
    ) {
        match self.handle_remote_fns() {
            // All the scenarios are OK.
            Ok(_)
            | Err(crossbeam_channel::TryRecvError::Empty)
            | Err(crossbeam_channel::TryRecvError::Disconnected) => (),
        };
        self.atom_sequence_input.clear();
        for (frame, data) in midi_in {
            self.atom_sequence_input
                .push_midi_event::<4>(frame, self.midi_urid, data)
                .unwrap();
        }
        let ports = livi::EmptyPortConnections::new()
            .with_audio_outputs(audio_out)
            .with_atom_sequence_inputs(std::iter::once(&self.atom_sequence_input));

        let res = self
            .plugin_instance
            .as_mut()
            .map(|i| unsafe { i.run(frames, ports) })
            .unwrap_or(Ok(()));
        if let Err(err) = res {
            // TODO: Drop this outside of processing thread.
            let p = self.plugin_instance.take().unwrap();
            error!("{:?}", err);
            error!("Disabling plugin {:?}.", p.raw().instance().uri());
        }
    }
}
