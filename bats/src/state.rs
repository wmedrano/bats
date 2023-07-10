use jack::AsyncClient;
use log::info;
use serde::{Deserialize, Serialize};

use crate::{bats::Bats, jack_adapter::JackProcessHandler, remote_executor::RemoteExecutor};

pub struct State {
    client: AsyncClient<(), JackProcessHandler>,
    executor: RemoteExecutor,
    world: livi::World,
}

impl State {
    pub fn new() -> Result<State, jack::Error> {
        let (client, status) = jack::Client::new("bats", jack::ClientOptions::NO_START_SERVER)?;
        info!(
            "Started JACK client {} with status {:?}",
            client.name(),
            status
        );
        let world = livi::World::new();
        let features = livi::FeaturesBuilder {
            min_block_length: 1,
            max_block_length: 2 * client.buffer_size() as usize,
        }
        .build(&world);
        let mut handler = JackProcessHandler::new(&client, &features)?;
        let executor = handler.bats.reset_remote_executor(8);

        let client = client.activate_async((), handler)?;
        Ok(State {
            client,
            executor,
            world,
        })
    }

    pub fn settings(&self) -> Settings {
        Settings {
            buffer_size: self.client.as_client().buffer_size() as usize,
            sample_rate: self.client.as_client().sample_rate() as f64,
        }
    }

    pub fn plugins(&self) -> Vec<Plugin> {
        self.world
            .iter_plugins()
            .map(|p| Plugin {
                id: PluginId {
                    namespace: "lv2".to_string(),
                    id: p.uri(),
                },
                name: p.name(),
            })
            .collect()
    }

    pub fn tracks(&self) -> Vec<Track> {
        let mut tracks = Vec::with_capacity(Bats::TRACKS_CAPACITY);
        self.executor
            .execute(move |b| {
                tracks.extend(b.tracks.iter().map(|t| Track {
                    enabled: t.enabled,
                    volume: t.volume,
                }));
                tracks
            })
            .unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    buffer_size: usize,
    sample_rate: f64,
}

#[derive(Serialize, Deserialize)]
pub struct Track {
    enabled: bool,
    volume: f32,
}

#[derive(Serialize, Deserialize)]
pub struct Plugin {
    id: PluginId,
    name: String,
}

#[derive(Serialize, Deserialize)]
pub struct PluginId {
    namespace: String,
    id: String,
}
