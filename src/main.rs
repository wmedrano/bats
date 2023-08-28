use anyhow::Result;
use log::*;

fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .try_init()
        .unwrap();
    let _client = make_client().unwrap();
    std::thread::park();
    info!("Exiting bats!");
}

fn make_client() -> Result<jack::AsyncClient<(), ()>> {
    let (client, status) = jack::Client::new("bats", jack::ClientOptions::empty())?;
    info!("Started client {} with status {:?}.", client.name(), status);
    let active_client = client.activate_async((), ())?;
    Ok(active_client)
}

#[cfg(test)]
#[test]
fn test_make_client_is_ok() {
    let client = make_client().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    client.deactivate().unwrap();
}
