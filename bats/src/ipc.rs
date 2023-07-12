use crate::bats::Bats;

/// Contains a callable function.
pub struct RawFn {
    /// A callable function.
    pub f: Box<dyn Send + FnOnce(&mut Bats)>,
}

/// A struct that can be used to communicate within a process.
pub struct Ipc {
    /// The channel to send functions to execute on.
    sender: crossbeam_channel::Sender<RawFn>,
}

impl Ipc {
    pub fn new(sender: crossbeam_channel::Sender<RawFn>) -> Ipc {
        Ipc { sender }
    }

    /// Call for `f` to be executed. This will send `f` to be executed but will not block.
    fn run_fn_async(&self, f: impl 'static + Send + FnOnce(&mut Bats)) {
        let raw_fn = RawFn {
            f: Box::new(move |s| {
                f(s);
            }),
        };
        self.sender.send(raw_fn).unwrap();
    }

    /// Execute `f` and return its value once it has executed. This function will block until the
    /// remote object has received and executed `f`.
    pub fn run_fn<T: 'static + Send>(
        &self,
        f: impl 'static + Send + FnOnce(&mut Bats) -> T,
    ) -> Result<T, crossbeam_channel::RecvError> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        self.run_fn_async(move |s| {
            let ret = f(s);
            tx.send(ret).unwrap();
        });
        rx.recv()
    }
}
