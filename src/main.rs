use anyhow::Result;
use log::info;

pub mod jack_adapter;

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .unwrap();
    info!(
        "Current Dir: {:?}, Args: {:?}",
        std::env::current_dir().unwrap(),
        std::env::args().collect::<Vec<_>>()
    );

    let (client, status) = jack::Client::new("bats", jack::ClientOptions::NO_START_SERVER)?;
    info!("Started client {} with status {:?}", client.name(), status);
    let ports = jack_adapter::Ports::new(&client)?;
    let process_handler = jack_adapter::ProcessHandler::new(ports);
    let client = client.activate_async((), process_handler)?;
    std::thread::park();
    client.deactivate()?;
    Ok(())
}
