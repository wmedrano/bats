use anyhow::Result;
use bats_lib::{
    plugin::{toof::Toof, BatsInstrument},
    Bats,
};
use clap::Parser;
use log::{error, info};

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

    let mut plugin_names = Vec::new();
    let mut bats = Bats::new(client.sample_rate() as f32, client.buffer_size() as usize);
    if args.load_initial_plugin {
        plugin_names.push(Toof::NAME.to_string());
        bats.add_plugin(Toof::new(bats.sample_rate()));
    }
    let process_handler = jack_adapter::ProcessHandler::new(&client, bats)?;
    let maybe_connector = if args.auto_connect_ports {
        Some(match process_handler.connector() {
            Ok(f) => f,
            Err(err) => {
                error!("Failed to create port connector! IO ports will have to be connected manually. Error: {}", err);
                Box::new(|| {})
            }
        })
    } else {
        None
    };

    let client = client.activate_async((), process_handler)?;
    if let Some(mut connector) = maybe_connector {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(1));
            connector();
        });
    }

    bats_ui::Ui::new(plugin_names)?.run()?;
    info!("Exiting bats!");
    client.deactivate()?;
    Ok(())
}
