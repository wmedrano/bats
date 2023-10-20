use anyhow::Result;
use log::{error, info};

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
    info!("Started JACK client {:?}.", client);
    info!("JACK status is {:?}", status);
    let process_handler = jack_adapter::ProcessHandler::new(&client)?;
    let mut connector = match process_handler.connector() {
        Ok(f) => f,
        Err(err) => {
            error!("Failed to create port connector! IO ports will have to be connected manually. Error: {}", err);
            Box::new(|| {})
        }
    };
    let client = client.activate_async((), process_handler)?;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        connector();
    });
    bats_ui::Ui::new()?.run()?;
    info!("Exiting bats!");
    client.deactivate()?;
    Ok(())
}
