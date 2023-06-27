use log::{error, info, warn};

mod process_handler;
mod readline;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    info!("{:?}", std::env::args());
    let world_handle = std::thread::spawn(livi::World::new);
    let (client, status) =
        jack::Client::new("simian-sonic", jack::ClientOptions::NO_START_SERVER).unwrap();
    let sample_rate = client.sample_rate() as f64;
    info!(
        "Created {}(sample_rate={sample_rate}) with status {status:?}.",
        client.name()
    );

    let world = world_handle.join().unwrap();
    let features = world.build_features(livi::FeaturesBuilder::default());
    let mut process_handler = process_handler::ProcessHandler::new(&client, &features).unwrap();
    let executor = process_handler.reset_remote_executor(1);
    if let Err(err) = process_handler.connect_ports(&client) {
        warn!("Failed to autoconnect ports: {:?}", err);
    };
    let active_client = client.activate_async((), process_handler).unwrap();

    let mut rl = readline::Readline::new().unwrap();
    info!("{}", readline::Command::help_str());
    loop {
        match rl.readline() {
            Err(err) => error!("{:?}", err),
            Ok(cmd) => {
                info!("Executing command: {:?}", cmd);
                match cmd {
                    readline::Command::ListPlugins => {
                        for (idx, p) in world.iter_plugins().enumerate() {
                            println!("{}: {}", idx, p.name());
                        }
                    }
                    readline::Command::SetPlugin(idx) => match world.iter_plugins().nth(idx) {
                        None => error!("plugin {} is not valid.", idx),
                        Some(p) => match unsafe { p.instantiate(features.clone(), sample_rate) } {
                            Ok(i) => {
                                let old: Option<livi::Instance> =
                                    executor.execute(move |ph| ph.plugin_instance.replace(i));
                                drop(old); // Drop outside main thread.
                            }
                            Err(err) => error!("{:?}", err),
                        },
                    },
                    readline::Command::Help => println!("{}", readline::Command::help_str()),
                    readline::Command::Nothing => (),
                    readline::Command::Exit => {
                        info!("Exiting...");
                        active_client.deactivate().unwrap();
                        return;
                    }
                }
            }
        }
    }
}
