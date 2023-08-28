use anyhow::Result;
use jack_adapter::JackAdapter;
use log::*;

mod jack_adapter;
mod processor;

fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
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
    let processor = JackAdapter::new(&client)?;
    let active_client = client.activate_async((), processor)?;
    Ok(active_client)
}

#[cfg(test)]
#[test]
fn test_make_client_is_ok() {
    let client = make_client().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    client.deactivate().unwrap();
}
