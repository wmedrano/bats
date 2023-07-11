use std::{
    collections::HashMap,
    sync::{atomic::AtomicU32, Arc},
};

use anyhow::{anyhow, Result};
use jack::AsyncClient;
use log::{info, warn};
use serde::{Deserialize, Serialize};

use crate::{bats::Bats, jack_adapter::JackProcessHandler, remote_executor::RemoteExecutor};

pub struct State {
    client: AsyncClient<(), JackProcessHandler>,
    executor: RemoteExecutor,
    world: livi::World,
    features: Arc<livi::Features>,
    urid_to_internal_id: HashMap<String, u32>,
    next_id: AtomicU32,
}

impl State {
    pub fn new() -> Result<State, jack::Error> {
        let mut next_id = 0u32;
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
        if let Err(err) = handler.connect_ports(&client) {
            warn!(
                "Failed to auto connect ports. There may be audio and/or midi IO issues: {:?}",
                err
            );
        };
        let executor = handler.bats.reset_remote_executor(8);
        let urid_to_internal_id = world
            .iter_plugins()
            .map(|p| {
                let entry = (p.uri(), next_id);
                next_id += 1;
                entry
            })
            .collect();

        let client = client.activate_async((), handler)?;
        Ok(State {
            client,
            executor,
            world,
            features,
            urid_to_internal_id,
            next_id: next_id.into(),
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

    pub fn make_track(&self) -> u32 {
        let id = self.claim_id();
        self.executor
            .execute(move |b| {
                b.tracks.push(crate::track::Track {
                    id,
                    plugin_instances: Vec::new(),
                    enabled: true,
                    volume: 0.5,
                });
            })
            .unwrap();
        id
    }

    pub fn delete_track(&self, id: IdParams) -> Result<()> {
        let id = id.id;
        let maybe_track = self
            .executor
            .execute(move |b| {
                let idx = b.tracks.iter().position(|t| t.id == id)?;
                Some(b.tracks.remove(idx))
            })
            .unwrap();
        match maybe_track {
            Some(_) => Ok(()),
            None => Err(anyhow!("could not find track with id {}", id)),
        }
    }

    pub fn tracks(&self) -> Vec<Track> {
        let mut tracks = Vec::with_capacity(Bats::TRACKS_CAPACITY);
        let mut tracks: Vec<Track> = self
            .executor
            .execute(move |b| {
                tracks.extend(b.tracks.iter().map(|t| Track {
                    id: t.id,
                    plugin_instances: Vec::new(),
                    enabled: t.enabled,
                    volume: t.volume,
                }));
                tracks
            })
            .unwrap();
        for plugin_instance in self.plugin_instances().into_iter() {
            if let Some(t) = tracks.iter_mut().find(|t| t.id == plugin_instance.track_id) {
                // The runtime of this operation is O(plugin_instances * tracks). If this becomes a problem,
                // this loop should be optimized.
                t.plugin_instances.push(plugin_instance);
            }
        }
        tracks
    }

    pub fn make_plugin_instance(&self, params: MakePluginInstanceParams) -> Result<PluginInstance> {
        let track_id = params.track_id;
        let plugin_id = params.plugin_id;
        let plugin_instance_id = self.claim_id();
        let (plugin, internal_plugin_id) = match plugin_id.namespace.as_str() {
            "lv2" => {
                let p = self
                    .world
                    .plugin_by_uri(&plugin_id.id)
                    .ok_or_else(|| anyhow!("could not find LV2 plugin {}", plugin_id.id))?;
                let id = *self.urid_to_internal_id.get(&plugin_id.id).ok_or_else(|| {
                    anyhow!("could not get internal id for LV2 plugin {}", plugin_id.id)
                })?;
                (p, id)
            }
            unknown => return Err(anyhow!("plugin namespace {} not known", unknown))?,
        };
        let plugin_instance = match unsafe {
            plugin.instantiate(
                self.features.clone(),
                self.client.as_client().sample_rate() as f64,
            )
        } {
            Ok(instance) => crate::track::PluginInstance {
                instance_id: plugin_instance_id,
                plugin_id: internal_plugin_id,
                instance,
            },
            Err(err) => return Err(anyhow!("failed to instantiate {:?}: {}", plugin_id, err)),
        };
        let failed_plugin_instance = self
            .executor
            .execute(move |b| {
                let track = match b.tracks.iter_mut().find(|t| t.id == track_id) {
                    Some(t) => t,
                    None => return Some(plugin_instance),
                };
                track.plugin_instances.push(plugin_instance);
                None
            })
            .unwrap();
        if failed_plugin_instance.is_some() {
            return Err(anyhow!("could not find track {}", track_id));
        }
        Ok(PluginInstance {
            id: plugin_instance_id,
            plugin_id,
            track_id,
        })
    }

    pub fn delete_plugin_instance(&self, id: IdParams) -> Result<()> {
        let id = id.id;
        let maybe_plugin_instance = self
            .executor
            .execute(move |b| {
                for track in b.tracks.iter_mut() {
                    if let Some(idx) = track
                        .plugin_instances
                        .iter()
                        .position(|i| i.instance_id == id)
                    {
                        return Some(track.plugin_instances.remove(idx));
                    }
                }
                None
            })
            .unwrap();
        match maybe_plugin_instance {
            Some(_) => Ok(()),
            None => Err(anyhow!("could not find plugin instance with id {}", id)),
        }
    }
    pub fn plugin_instances(&self) -> Vec<PluginInstance> {
        let mut plugin_instances =
            Vec::with_capacity(Bats::TRACKS_CAPACITY * Bats::PLUGIN_INSTANCE_CAPACITY);
        struct PluginInstanceTmp {
            plugin_instance: PluginInstance,
            internal_plugin_id: u32,
        }
        let plugin_instances_tmp = self
            .executor
            .execute(move |b| {
                plugin_instances.extend(
                    b.tracks
                        .iter()
                        .flat_map(|t| t.plugin_instances.iter().map(|pi| (t.id, pi)))
                        .map(|(track_id, pi)| PluginInstanceTmp {
                            plugin_instance: PluginInstance {
                                id: pi.instance_id,
                                plugin_id: PluginId::default(),
                                track_id,
                            },
                            internal_plugin_id: pi.plugin_id,
                        }),
                );
                plugin_instances
            })
            .unwrap();
        plugin_instances_tmp
            .into_iter()
            .map(|tmp| PluginInstance {
                id: tmp.plugin_instance.id,
                track_id: tmp.plugin_instance.track_id,
                plugin_id: PluginId {
                    namespace: "lv2".to_string(),
                    id: self
                        .urid_to_internal_id
                        .iter()
                        .find(|(_, v)| **v == tmp.internal_plugin_id)
                        .map(|(k, _)| k.clone())
                        .unwrap(),
                },
            })
            .collect()
    }
}

impl State {
    fn claim_id(&self) -> u32 {
        self.next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MakePluginInstanceParams {
    track_id: u32,
    plugin_id: PluginId,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct IdParams {
    id: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    buffer_size: usize,
    sample_rate: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Track {
    id: u32,
    plugin_instances: Vec<PluginInstance>,
    enabled: bool,
    volume: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PluginInstance {
    id: u32,
    plugin_id: PluginId,
    track_id: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Plugin {
    id: PluginId,
    name: String,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct PluginId {
    namespace: String,
    id: String,
}
