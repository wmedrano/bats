//! Bats! is a sample centered DAW.
use anyhow::Result;
use jack_adapter::JackAdapter;
use log::*;

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
    let client = make_client().unwrap();
    std::thread::park();
    info!("Exiting bats!");
    client.deactivate().unwrap();
}

/// Make a new client or return of error.
fn make_client() -> Result<jack::AsyncClient<(), JackAdapter>> {
    let (client, status) = jack::Client::new("bats", jack::ClientOptions::NO_START_SERVER)?;
    info!("Started client {} with status {:?}.", client.name(), status);
    let (processor, mut communicator) = JackAdapter::new(&client)?;
    if let Err(err) = processor.connect_ports(&client) {
        warn!("{:?}", err);
    }
    let active_client = client.activate_async((), processor)?;
    let sampler_plugin = Plugin::OneShotSampler(OneShotSampler::new(Sample::with_wave_file(
        "assets/LoFi-drum-loop.wav",
    )?));
    communicator.call(move |p| p.plugins.push(sampler_plugin));
    Ok(active_client)
}

#[cfg(test)]
#[test]
fn test_main_make_client_is_ok() {
    let client = make_client().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    client.deactivate().unwrap();
}
