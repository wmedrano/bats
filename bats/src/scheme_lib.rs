use std::{ffi::CStr, sync::Arc};

use crate::{
    bats::Bats,
    jack_adapter::JackProcessHandler,
    remote_executor::RemoteExecutor,
    track::{PluginInstance, Track},
};
use flashkick::Scm;
use lazy_static::lazy_static;
use log::{info, warn};

/// Register all scheme functions.
///
/// # Safety
/// Registers functions with scheme which may be unsafe.
#[no_mangle]
pub unsafe extern "C" fn init_bats() {
    define_subr(
        CStr::from_bytes_with_nul(b"activate-logging!\0").unwrap(),
        0,
        0,
        0,
        activate_logging as _,
    );
    define_subr(
        CStr::from_bytes_with_nul(b"settings\0").unwrap(),
        0,
        0,
        0,
        settings as _,
    );
    define_subr(
        CStr::from_bytes_with_nul(b"plugins\0").unwrap(),
        0,
        0,
        0,
        plugins as _,
    );
    define_subr(
        CStr::from_bytes_with_nul(b"make-track!\0").unwrap(),
        0,
        0,
        1,
        make_track as _,
    );
    define_subr(
        CStr::from_bytes_with_nul(b"make-plugin-instance!\0").unwrap(),
        2,
        0,
        0,
        make_plugin_instance as _,
    );
    define_subr(
        CStr::from_bytes_with_nul(b"plugin-instance\0").unwrap(),
        1,
        0,
        0,
        plugin_instance as _,
    );
    define_subr(
        CStr::from_bytes_with_nul(b"delete-track!\0").unwrap(),
        1,
        0,
        0,
        delete_track as _,
    );
    define_subr(
        CStr::from_bytes_with_nul(b"delete-plugin-instance!\0").unwrap(),
        1,
        0,
        0,
        delete_plugin_instance as _,
    );
    define_subr(
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
    Scm::with_alist(
        [
            (
                Scm::new_symbol("buffer-size"),
                Scm::new_u32(state.client.as_client().buffer_size()),
            ),
            (
                Scm::new_symbol("sample-rate"),
                Scm::new_u32(state.client.as_client().sample_rate() as u32),
            ),
            (
                Scm::new_symbol("cpu-load"),
                Scm::new_f64(state.client.as_client().cpu_load() as f64),
            ),
            (
                Scm::new_symbol("client-name"),
                Scm::new_string(state.client.as_client().name()),
            ),
        ]
        .into_iter(),
    )
}

unsafe extern "C" fn plugins() -> Scm {
    let name_key = Scm::new_symbol("name");
    let plugin_id_key = Scm::new_symbol("plugin-id");
    let is_instrument_key = Scm::new_symbol("instrument?");
    let classes_key = Scm::new_symbol("classes");
    let lv2_sym = Scm::new_symbol("lv2");
    Scm::with_reversed_list(STATE.world.iter_plugins().map(move |p| {
        Scm::with_alist(
            [
                (is_instrument_key, Scm::new_bool(p.is_instrument())),
                (name_key, Scm::new_string(p.name().as_str())),
                (
                    plugin_id_key,
                    Scm::new_pair(lv2_sym, Scm::new_string(&p.uri())),
                ),
                (
                    classes_key,
                    Scm::with_reversed_list(p.classes().map(|c| Scm::new_string(c))),
                ),
            ]
            .into_iter(),
        )
    }))
}

unsafe fn scm_to_plugin_instance(state: &State, plugin_id: Scm) -> PluginInstance {
    let error_key = Scm::new_symbol("instantiate-plugin-error");
    let subr = CStr::from_bytes_with_nul(b"make-plugin-instance!\0").unwrap();
    let plugin_ns = plugin_id.car().to_symbol();
    let plugin_uri = plugin_id.cdr().to_string();
    if plugin_ns != "lv2" {
        scm_error(
            error_key,
            subr,
            CStr::from_bytes_with_nul(b"Only type lv2 is supported but got ~S.\0").unwrap(),
            Scm::with_reversed_list(std::iter::once(plugin_id.car())),
            Scm::FALSE,
        );
    }
    let plugin = match state.world.plugin_by_uri(&plugin_uri) {
        Some(p) => p,
        None => {
            scm_error(
                error_key,
                subr,
                CStr::from_bytes_with_nul(b"lv2 plugin with URI ~s not found.\0").unwrap(),
                Scm::with_reversed_list(std::iter::once(plugin_id.cdr())),
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
            scm_error(
                Scm::EOL,
                subr,
                CStr::from_bytes_with_nul(b"Failed to instantiate plugin ~S.\0").unwrap(),
                Scm::with_reversed_list(std::iter::once(Scm::new_string(&err.to_string()))),
                Scm::FALSE,
            );
        }
    }
}

unsafe extern "C" fn make_plugin_instance(track_id: Scm, plugin_id: Scm) -> Scm {
    let state = &*STATE;
    let track_id = track_id.to_u32();
    let plugin_instance = scm_to_plugin_instance(state, plugin_id);
    let plugin_instance_id = plugin_instance.instance_id;
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
    if did_add {
        Scm::new_u32(plugin_instance_id)
    } else {
        Scm::FALSE
    }
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
                        .any(|i| i.instance_id == plugin_instance_id)
                })
                .map(|t| t.id)
        })
        .unwrap()
        .unwrap_or_else(|| {
            let error_key = Scm::new_symbol("not-found");
            let subr = CStr::from_bytes_with_nul(b"track-id-for-plugin-instance\0").unwrap();
            scm_error(
                error_key,
                subr,
                CStr::from_bytes_with_nul(b"Plugin instance ~S not found.\0").unwrap(),
                Scm::with_reversed_list(std::iter::once(Scm::new_u32(plugin_instance_id))),
                Scm::FALSE,
            );
        })
}

unsafe extern "C" fn plugin_instance(plugin_instance_id: Scm) -> Scm {
    let lv2_sym = Scm::new_symbol("lv2");
    let state = &*STATE;
    let plugin_instance_id = plugin_instance_id.to_u32();
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
    Scm::with_alist(
        [
            (Scm::new_symbol("track-id"), Scm::new_u32(info.track_id)),
            (
                Scm::new_symbol("plugin-id"),
                Scm::new_pair(lv2_sym, Scm::new_string(&uri)),
            ),
            (
                Scm::new_symbol("plugin-instance"),
                Scm::new_u32(info.plugin_instance_id),
            ),
        ]
        .into_iter(),
    )
}

unsafe extern "C" fn make_track(rest: Scm) -> Scm {
    let enabled_keyword = Scm::new_keyword("enabled");
    let volume_keyword = Scm::new_keyword("volume");
    let plugins_keyword = Scm::new_keyword("plugin-ids");
    let mut enabled = Scm::TRUE;
    let mut volume = Scm::new_f64(0.5f64);
    let mut plugins = Scm::EOL;
    flashkick::ffi::scm_c_bind_keyword_arguments(
        CStr::from_bytes_with_nul(b"make-track\0").unwrap().as_ptr(),
        rest.0,
        0,
        enabled_keyword.0,
        &mut enabled,
        volume_keyword.0,
        &mut volume,
        plugins_keyword,
        &mut plugins,
        Scm::EOL.0,
    );
    let state = &*STATE;
    let id = state
        .next_id
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let track = Track {
        id,
        plugin_instances: {
            let mut ret = Vec::with_capacity(Bats::PLUGIN_INSTANCE_CAPACITY);
            ret.extend(plugins.iter().map(|p| scm_to_plugin_instance(state, p)));
            ret
        },
        enabled: enabled.to_bool(),
        volume: volume.to_f64() as f32,
    };
    STATE
        .executor
        .execute(move |s| {
            s.tracks.push(track);
        })
        .unwrap();
    Scm::new_u32(id)
}

unsafe extern "C" fn delete_track(id: Scm) -> Scm {
    let id = id.to_u32();
    let maybe_track = STATE
        .executor
        .execute(move |s| -> Option<Track> {
            let idx = s.tracks.iter().position(|t| t.id == id)?;
            Some(s.tracks.remove(idx))
        })
        .unwrap();
    Scm::new_bool(maybe_track.is_some())
}

unsafe extern "C" fn delete_plugin_instance(plugin_instance_id: Scm) -> Scm {
    let state = &*STATE;
    let plugin_instance_id = plugin_instance_id.to_u32();
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
    let track_id_key = Scm::new_symbol("track-id");
    let volume_key = Scm::new_symbol("volume");
    let enabled_key = Scm::new_symbol("enabled?");
    let plugin_instance_ids_key = Scm::new_symbol("plugin-instance-ids");
    Scm::with_reversed_list(tracks.into_iter().map(|t| {
        Scm::with_alist(
            [
                (
                    plugin_instance_ids_key,
                    Scm::with_reversed_list(
                        t.plugin_instance_ids
                            .iter()
                            .rev()
                            .map(|id| Scm::new_u32(*id)),
                    ),
                ),
                (volume_key, Scm::new_f64(t.volume as f64)),
                (enabled_key, Scm::new_bool(t.enabled)),
                (track_id_key, Scm::new_u32(t.id)),
            ]
            .into_iter(),
        )
    }))
}

/// Define a subroutine.
///
/// `name` - The name of the subroutine.
/// `req`  - The number of required arguments.
/// `opt`  - The number of optional arguments.
/// `rst`  - The number of rest arguments.
/// `fcn`  - The function implementation. The function must be of type
///          `extern "C"` or `unsafe extern "C"`. It must take the
///          appropriate amount of `Scm` as arguments and return a
///          single `Scm` object.
///
/// # Safety
/// Undefined behavior if `fcn` does not have the right signature.
pub unsafe fn define_subr(
    name: &CStr,
    req: usize,
    opt: usize,
    rst: usize,
    fcn: flashkick::ffi::scm_t_subr,
) {
    flashkick::ffi::scm_c_define_gsubr(name.as_ptr(), req as _, opt as _, rst as _, fcn);
}

/// Raises a Scheme error. This is similar to a Rust panic.
///
/// # Safety
/// Uses unsafe functions.
pub unsafe fn scm_error(k: Scm, subr: &CStr, message: &CStr, args: Scm, rest: Scm) -> ! {
    flashkick::ffi::scm_error(k.0, subr.as_ptr(), message.as_ptr(), args.0, rest.0)
}
