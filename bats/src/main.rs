use anyhow::Result;
use bats_async::new_async_commander;
use bats_dsp::SampleRate;
use bats_lib::{
    plugin::{toof::Toof, BatsInstrument},
    Bats, PluginInstance,
};
use clap::Parser;
use log::{error, info};

use crate::jack_adapter::NotificationHandler;

pub mod args;
pub mod jack_adapter;

fn main() -> Result<()> {
    let args = args::Args::parse();
    info!("Pared args: {:?}", args);

    env_logger::builder()
        .filter_level(args.log_level)
        .try_init()
        .unwrap();
    info!("Current Dir: {:?}", std::env::current_dir().unwrap(),);
    info!("Raw args: {:?}", std::env::args());
    info!("Pared args: {:?}", args);

    let (client, status) = jack::Client::new("bats", jack::ClientOptions::NO_START_SERVER)?;
    info!("Started JACK client {:?}.", client);
    info!("JACK status is {:?}", status);

    let bats = make_bats(&client, args.load_initial_plugin);
    let (command_sender, command_receiver) = new_async_commander();
    let mut ui = bats_ui::Ui::new(&bats, command_sender)?;
    let process_handler = jack_adapter::ProcessHandler::new(&client, bats, command_receiver)?;
    let maybe_connector = maybe_make_connector(&process_handler, args.auto_connect_ports);
    let client = client.activate_async(NotificationHandler {}, process_handler)?;
    spawn_connector_daemon(maybe_connector);

    ui.run()?;
    info!("Exiting bats!");
    client.deactivate()?;
    Ok(())
}

fn make_bats(client: &jack::Client, load_initial_plugin: bool) -> Bats {
    let sample_rate = SampleRate::new(client.sample_rate() as f32);
    let buffer_size = client.buffer_size() as usize;
    let mut bats = Bats::new(sample_rate, client.buffer_size() as usize);
    if load_initial_plugin {
        bats.plugins.push(PluginInstance {
            id: 0,
            plugin: Toof::new(sample_rate),
            output: bats_dsp::buffers::Buffers::new(buffer_size),
        });
    }
    bats
}

fn maybe_make_connector(
    process_handler: &jack_adapter::ProcessHandler,
    enable_connector: bool,
) -> Option<Box<dyn Send + FnMut()>> {
    if enable_connector {
        Some(match process_handler.connector() {
            Ok(f) => f,
            Err(err) => {
                error!("Failed to create port connector! IO ports will have to be connected manually. Error: {}", err);
                Box::new(|| {})
            }
        })
    } else {
        None
    }
}

fn spawn_connector_daemon(connector: Option<Box<dyn Send + FnMut()>>) {
    if let Some(mut connector) = connector {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(1));
            loop {
                connector();
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        });
    }
}
