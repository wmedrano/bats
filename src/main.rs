use std::sync::Arc;

use log::{error, info, warn};

mod jack_adapter;
mod readline;
mod remote_executor;
mod simian;
mod track;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    info!("{:?}", std::env::args());
    let world_and_features = std::thread::spawn(new_world_and_features);
    let (client, status) =
        jack::Client::new("simian-sonic", jack::ClientOptions::NO_START_SERVER).unwrap();
    let sample_rate = client.sample_rate() as f64;
    info!(
        "Created {}(sample_rate={sample_rate}) with status {status:?}.",
        client.name()
    );

    let (world, features) = world_and_features.join().unwrap();
    let mut process_handler = jack_adapter::JackProcessHandler::new(&client, &features).unwrap();
    let executor = process_handler.simian.reset_remote_executor(1);
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
                    readline::Command::AddTrack(plugin_index) => match world
                        .iter_plugins()
                        .nth(plugin_index)
                    {
                        None => error!("plugin {} is not valid.", plugin_index),
                        Some(p) => match unsafe { p.instantiate(features.clone(), sample_rate) } {
                            Ok(plugin_instance) => {
                                let mut plugin_instances = Vec::with_capacity(16);
                                plugin_instances.push(plugin_instance);
                                let track = track::Track {
                                    plugin_instances,
                                    enabled: true,
                                    volume: 0.25,
                                };
                                executor.execute(move |s| s.tracks.push(track)).unwrap();
                            }
                            Err(err) => error!("{:?}", err),
                        },
                    },
                    readline::Command::AddPlugin { track, plugin } => match world
                        .iter_plugins()
                        .nth(plugin)
                    {
                        None => error!("plugin {} is not valid.", plugin),
                        Some(p) => match unsafe { p.instantiate(features.clone(), sample_rate) } {
                            Ok(plugin_instance) => {
                                executor
                                    .execute(move |s| {
                                        // TODO: Check bounds.
                                        s.tracks[track].plugin_instances.push(plugin_instance)
                                    })
                                    .unwrap();
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

fn new_world_and_features() -> (livi::World, Arc<livi::Features>) {
    let world = livi::World::new();
    let features = world.build_features(livi::FeaturesBuilder::default());
    (world, features)
}
