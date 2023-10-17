use anyhow::Result;
use log::info;

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
    let process_handler = ();
    let client = client.activate_async((), process_handler)?;
    std::thread::park();
    client.deactivate()?;
    Ok(())
}
