use std::{ffi::CStr, sync::Arc};

use crate::{
    bats::Bats,
    jack_adapter::JackProcessHandler,
    remote_executor::RemoteExecutor,
    track::{PluginInstance, Track},
};
use flashkick::{FromScm, Scm, ToScm};
use lazy_static::lazy_static;
use log::{info, warn};

/// Register all scheme functions.
///
/// # Safety
/// Registers functions with scheme which may be unsafe.
#[no_mangle]
pub unsafe extern "C" fn init_bats() {
    flashkick::define_subr(
        CStr::from_bytes_with_nul(b"activate-logging!\0").unwrap(),
        0,
        0,
        0,
        activate_logging as _,
    );
    flashkick::define_subr(
        CStr::from_bytes_with_nul(b"settings\0").unwrap(),
        0,
        0,
        0,
        settings as _,
    );
    flashkick::define_subr(
        CStr::from_bytes_with_nul(b"plugins\0").unwrap(),
        0,
        0,
        0,
        plugins as _,
    );
    flashkick::define_subr(
        CStr::from_bytes_with_nul(b"make-track!\0").unwrap(),
        0,
        0,
        1,
        make_track as _,
    );
    flashkick::define_subr(
        CStr::from_bytes_with_nul(b"make-plugin-instance!\0").unwrap(),
        2,
        0,
        0,
        make_plugin_instance as _,
    );
    flashkick::define_subr(
        CStr::from_bytes_with_nul(b"plugin-instance\0").unwrap(),
        1,
        0,
        0,
        plugin_instance as _,
    );
    flashkick::define_subr(
        CStr::from_bytes_with_nul(b"delete-track!\0").unwrap(),
        1,
        0,
        0,
        delete_track as _,
    );
    flashkick::define_subr(
        CStr::from_bytes_with_nul(b"delete-plugin-instance!\0").unwrap(),
        1,
        0,
        0,
        delete_plugin_instance as _,
    );
    flashkick::define_subr(
        CStr::from_bytes_with_nul(b"tracks\0").unwrap(),
        0,
        0,
        0,
        tracks as _,
    );
}

struct State {
    executor: RemoteExecutor,
    world: livi::World,
    urid_to_id: Vec<(String, u32)>,
    features: Arc<livi::Features>,
    client: jack::AsyncClient<(), JackProcessHandler>,
    next_id: std::sync::atomic::AtomicU32,
}

lazy_static! {
    static ref STATE: State = {
        let (client, status) =
            jack::Client::new("bats", jack::ClientOptions::NO_START_SERVER).unwrap();
        let sample_rate = client.sample_rate() as f64;
        info!(
            "Created {}(sample_rate={sample_rate}) with status {status:?}.",
            client.name()
        );

        let mut next_id = 1;
        let world = livi::World::new();
        let urid_to_id = {
            let mut m = Vec::new();
            for plugin in world.iter_plugins() {
                m.push((plugin.uri(), next_id));
                next_id += 1;
            }
            m
        };
        let features = livi::FeaturesBuilder {
            min_block_length: 1,
            max_block_length: client.buffer_size() as usize * 2,
        }
        .build(&world);
        let mut process_handler = JackProcessHandler::new(&client, &features).unwrap();
        let executor = process_handler.bats.reset_remote_executor(1);
        if let Err(err) = process_handler.connect_ports(&client) {
            warn!("Failed to autoconnect ports: {:?}", err);
        };
        let client = client.activate_async((), process_handler).unwrap();
        State {
            executor,
            world,
            urid_to_id,
            features,
            client,
            next_id: next_id.into(),
        }
    };
}

impl State {
    fn claim_id(&self) -> u32 {
        self.next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

unsafe extern "C" fn activate_logging() -> Scm {
    match env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
    {
        Ok(()) => info!("Logging enabled."),
        Err(err) => warn!("Failed to initialize logging: {}", err),
    }
    Scm::EOL
}

unsafe extern "C" fn settings() -> Scm {
    let state = &*STATE;
    Scm::EOL
        .acons(
            Scm::with_symbol("buffer-size"),
            state.client.as_client().buffer_size(),
        )
        .acons(
            Scm::with_symbol("sample-rate"),
            state.client.as_client().sample_rate() as u32,
        )
        .acons(
            Scm::with_symbol("cpu-load"),
            state.client.as_client().cpu_load(),
        )
        .acons(
            Scm::with_symbol("client-name"),
            state.client.as_client().name(),
        )
}

unsafe extern "C" fn plugins() -> Scm {
    let name_key = Scm::with_symbol("name");
    let plugin_id_key = Scm::with_symbol("plugin-id");
    let is_instrument_key = Scm::with_symbol("instrument?");
    let classes_key = Scm::with_symbol("classes");
    let lv2_sym = Scm::with_symbol("lv2");
    Scm::with_list(STATE.world.iter_plugins().map(move |p| {
        Scm::EOL
            .acons(is_instrument_key, p.is_instrument())
            .acons(name_key, p.name().as_str())
            .acons(plugin_id_key, Scm::new(p.uri()).cons(lv2_sym))
            .acons(classes_key, Scm::with_list(p.classes().map(|c| c.to_scm())))
    }))
}

unsafe fn scm_to_plugin_instance(state: &State, plugin_id: Scm) -> PluginInstance {
    let error_key = Scm::with_symbol("instantiate-plugin-error");
    let subr = CStr::from_bytes_with_nul(b"make-plugin-instance!\0").unwrap();
    let plugin_ns = String::from_scm(plugin_id.car().symbol_to_str().to_scm());
    let plugin_uri = String::from_scm(plugin_id.cdr());
    if plugin_ns != "lv2" {
        flashkick::scm_error(
            error_key,
            subr,
            CStr::from_bytes_with_nul(b"Only type lv2 is supported but got ~S.\0").unwrap(),
            Scm::with_list(std::iter::once(plugin_id.list_ref(0))),
            Scm::FALSE,
        );
    }
    let plugin = match state.world.plugin_by_uri(&plugin_uri) {
        Some(p) => p,
        None => {
            flashkick::scm_error(
                error_key,
                subr,
                CStr::from_bytes_with_nul(b"lv2 plugin with URI ~s not found.\0").unwrap(),
                Scm::with_list(std::iter::once(plugin_id.list_ref(1))),
                Scm::FALSE,
            );
        }
    };
    match plugin.instantiate(
        state.features.clone(),
        state.client.as_client().sample_rate() as f64,
    ) {
        Ok(instance) => PluginInstance {
            instance_id: state.claim_id(),
            plugin_id: state
                .urid_to_id
                .iter()
                .find(|(uri, _)| uri == plugin_uri.as_str())
                .unwrap()
                .1,
            instance,
        },
        Err(err) => {
            flashkick::scm_error(
                Scm::EOL,
                subr,
                CStr::from_bytes_with_nul(b"Failed to instantiate plugin ~S.\0").unwrap(),
                Scm::with_list(std::iter::once(Scm::new(err.to_string()))),
                Scm::FALSE,
            );
        }
    }
}

unsafe extern "C" fn make_plugin_instance(track_id: Scm, plugin_id: Scm) -> Scm {
    let state = &*STATE;
    let track_id = u32::from_scm(track_id);
    let plugin_instance = scm_to_plugin_instance(state, plugin_id);
    let did_add = STATE
        .executor
        .execute(
            move |s| match s.tracks.iter_mut().find(|t| t.id == track_id) {
                None => false,
                Some(t) => {
                    t.plugin_instances.push(plugin_instance);
                    true
                }
            },
        )
        .unwrap();
    did_add.to_scm()
}

unsafe fn track_id_for_plugin_instance(state: &State, plugin_instance_id: u32) -> u32 {
    state
        .executor
        .execute(move |b| {
            b.tracks
                .iter()
                .find(|t| {
                    t.plugin_instances
                        .iter()
                        .find(|i| i.instance_id == plugin_instance_id)
                        .is_some()
                })
                .map(|t| t.id)
        })
        .unwrap()
        .unwrap_or_else(|| {
            let error_key = Scm::with_symbol("not-found");
            let subr = CStr::from_bytes_with_nul(b"track-id-for-plugin-instance\0").unwrap();
            flashkick::scm_error(
                error_key,
                subr,
                CStr::from_bytes_with_nul(b"Plugin instance ~S not found.\0").unwrap(),
                Scm::with_list(std::iter::once(Scm::new(plugin_instance_id))),
                Scm::FALSE,
            );
        })
}

unsafe extern "C" fn plugin_instance(plugin_instance_id: Scm) -> Scm {
    let lv2_sym = Scm::with_symbol("lv2");
    let state = &*STATE;
    let plugin_instance_id = u32::from_scm(plugin_instance_id);
    let track_id = track_id_for_plugin_instance(state, plugin_instance_id);
    struct PluginInstanceInfo {
        track_id: u32,
        plugin_instance_id: u32,
        plugin_id: u32,
    }
    let info = state
        .executor
        .execute(move |s| -> PluginInstanceInfo {
            let track = s.tracks.iter_mut().find(|t| t.id == track_id).unwrap();
            let plugin_instance = track
                .plugin_instances
                .iter()
                .find(|pi| pi.instance_id == plugin_instance_id)
                .unwrap();
            PluginInstanceInfo {
                track_id,
                plugin_instance_id,
                plugin_id: plugin_instance.plugin_id,
            }
        })
        .unwrap();
    let uri = state
        .urid_to_id
        .iter()
        .find(|(_, id)| *id == info.plugin_id)
        .map(|(uri, _)| uri.clone())
        .unwrap();
    Scm::EOL
        .acons(Scm::with_symbol("track-id"), Scm::new(info.track_id))
        .acons(Scm::with_symbol("plugin-id"), Scm::new(uri).cons(lv2_sym))
        .acons(
            Scm::with_symbol("plugin-instance"),
            Scm::new(info.plugin_instance_id),
        )
}

unsafe extern "C" fn make_track(rest: Scm) -> Scm {
    let enabled_keyword = Scm::with_keyword("enabled");
    let volume_keyword = Scm::with_keyword("volume");
    let plugins_keyword = Scm::with_keyword("plugin-ids");
    let mut enabled = Scm::TRUE;
    let mut volume = Scm::new(0.5f32);
    let mut plugins = Scm::EOL;
    flashkick::scm_c_bind_keyword_arguments(
        CStr::from_bytes_with_nul(b"make-track\0").unwrap().as_ptr(),
        rest.raw(),
        0,
        enabled_keyword.raw(),
        &mut enabled,
        volume_keyword.raw(),
        &mut volume,
        plugins_keyword,
        &mut plugins,
        Scm::UNDEFINED.raw(),
    );
    let state = &*STATE;
    let id = state
        .next_id
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let track = Track {
        id,
        plugin_instances: {
            let mut ret = Vec::with_capacity(Bats::PLUGIN_INSTANCE_CAPACITY);
            ret.extend(
                plugins
                    .iter_list()
                    .map(|p| scm_to_plugin_instance(state, p)),
            );
            ret
        },
        enabled: bool::from_scm(enabled),
        volume: f32::from_scm(volume),
    };
    STATE
        .executor
        .execute(move |s| {
            s.tracks.push(track);
        })
        .unwrap();
    Scm::new(id)
}

unsafe extern "C" fn delete_track(id: Scm) -> Scm {
    let id = u32::from_scm(id);
    let maybe_track = STATE
        .executor
        .execute(move |s| -> Option<Track> {
            let idx = s.tracks.iter().position(|t| t.id == id)?;
            Some(s.tracks.remove(idx))
        })
        .unwrap();
    Scm::new(maybe_track.is_some())
}

unsafe extern "C" fn delete_plugin_instance(plugin_instance_id: Scm) -> Scm {
    let state = &*STATE;
    let plugin_instance_id = u32::from_scm(plugin_instance_id);
    let track_id = track_id_for_plugin_instance(state, plugin_instance_id);
    let _ = state.executor.execute(move |s| -> PluginInstance {
        let track = s.tracks.iter_mut().find(|t| t.id == track_id).unwrap();
        let idx = track
            .plugin_instances
            .iter()
            .position(|pi| pi.instance_id == plugin_instance_id)
            .unwrap();
        track.plugin_instances.remove(idx)
    });
    Scm::EOL
}

unsafe extern "C" fn tracks() -> Scm {
    struct TrackInfo {
        id: u32,
        plugin_instance_ids: Vec<u32>,
        volume: f32,
        enabled: bool,
    }
    let mut tracks = Vec::with_capacity(Bats::TRACKS_CAPACITY);
    let tracks = STATE
        .executor
        .execute(move |s| -> Vec<TrackInfo> {
            tracks.extend(s.tracks.iter().map(|t| TrackInfo {
                id: t.id,
                // TODO: Do not allocate memory here.
                plugin_instance_ids: t.plugin_instances.iter().map(|i| i.instance_id).collect(),
                volume: t.volume,
                enabled: t.enabled,
            }));
            tracks
        })
        .unwrap();
    let track_id_key = Scm::with_symbol("track-id");
    let volume_key = Scm::with_symbol("volume");
    let enabled_key = Scm::with_symbol("enabled?");
    let plugin_instance_ids_key = Scm::with_symbol("plugin-instance-ids");
    Scm::with_list(tracks.into_iter().map(|t| {
        Scm::EOL
            .acons(
                plugin_instance_ids_key,
                Scm::with_list(t.plugin_instance_ids.iter().map(|id| Scm::new(*id))),
            )
            .acons(volume_key, t.volume)
            .acons(enabled_key, t.enabled)
            .acons(track_id_key, t.id)
    }))
}
