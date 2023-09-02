//! Bats! is a sample centered DAW.
use anyhow::Result;
use jack_adapter::JackAdapter;
use log::*;
use processor::ProcessorCommunicator;

use crate::{
    plugins::{sampler::OneShotSampler, Plugin},
    sample::Sample,
};

pub mod jack_adapter;
pub mod plugins;
pub mod processor;
pub mod sample;

/// Run the bats! program.
fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .unwrap();
    info!(
        "Current Dir: {:?}, Args: {:?}",
        std::env::current_dir().unwrap(),
        std::env::args().collect::<Vec<_>>()
    );
    let client = ProcessorClient::new().unwrap();
    std::thread::park();
    info!("Exiting bats!");
    client.deactivate().unwrap();
}

#[derive(Debug)]
pub struct ProcessorClient {
    pub active_client: jack::AsyncClient<(), JackAdapter>,
    pub communicator: ProcessorCommunicator,
}

impl ProcessorClient {
    /// Make a new client or return an error.
    pub fn new() -> Result<ProcessorClient> {
        let (client, status) = jack::Client::new("bats", jack::ClientOptions::NO_START_SERVER)?;
        info!("Started client {} with status {:?}.", client.name(), status);
        let (processor, communicator) = JackAdapter::new(&client)?;
        let connect_fn = processor.connect_ports_fn();
        let mut processor_client = ProcessorClient {
            active_client: client.activate_async((), processor)?,
            communicator,
        };
        let sampler_plugin = Plugin::OneShotSampler(OneShotSampler::new(Sample::with_wave_file(
            "assets/LoFi-drum-loop.wav",
        )?));
        processor_client
            .communicator
            .call(move |p| p.plugins.push(sampler_plugin));
        if let Err(err) = (connect_fn)() {
            warn!("{:?}", err);
        }
        Ok(processor_client)
    }

    /// Deactivate the client.
    pub fn deactivate(self) -> Result<()> {
        self.active_client.deactivate()?;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_main_make_client_is_ok() {
    let client = ProcessorClient::new().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    client.deactivate().unwrap();
}
