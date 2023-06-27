use crate::simian::Simian;

/// Contains a callable function.
pub struct RawFn {
    /// A callable function.
    pub f: Box<dyn Send + FnOnce(&mut Simian)>,
}

/// A struct that can execute code on an object that is running on a different thread.
pub struct RemoteExecutor {
    /// The channel to send functions to execute on.
    sender: crossbeam_channel::Sender<RawFn>,
}

impl RemoteExecutor {
    pub fn new(sender: crossbeam_channel::Sender<RawFn>) -> RemoteExecutor {
        RemoteExecutor { sender }
    }

    /// Call for `f` to be executed. This will send `f` to be executed but will not block.
    fn execute_async(&self, f: impl 'static + Send + FnOnce(&mut Simian)) {
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
        self.execute_async(move |s| {
            let ret = f(s);
            tx.send(ret).unwrap();
        });
        rx.recv()
    }
}
