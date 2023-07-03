use std::{ffi::CStr, sync::Arc};

use crate::{jack_adapter::JackProcessHandler, remote_executor::RemoteExecutor, track::Track};
use flashkick::{FromScm, Scm, ToScm};
use lazy_static::lazy_static;
use log::{error, info, warn};

/// Register all scheme functions.
#[no_mangle]
pub unsafe extern "C" fn init_bats() {
    flashkick::define_subr(
        CStr::from_bytes_with_nul(b"ensure-init\0").unwrap(),
        0,
        0,
        0,
        ensure_init as _,
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
        CStr::from_bytes_with_nul(b"make-track\0").unwrap(),
        0,
        0,
        0,
        make_track as _,
    );
    flashkick::define_subr(
        CStr::from_bytes_with_nul(b"instantiate-plugin\0").unwrap(),
        2,
        0,
        0,
        instantiate_plugin as _,
    );
    flashkick::define_subr(
        CStr::from_bytes_with_nul(b"delete-track\0").unwrap(),
        1,
        0,
        0,
        delete_track as _,
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

        let world = livi::World::new();
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
            features,
            client,
            next_id: 1.into(),
        }
    };
}

unsafe extern "C" fn ensure_init() -> Scm {
    let _ = &*STATE;
    Scm::TRUE
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
    let id_key = Scm::with_symbol("id");
    let is_instrument_key = Scm::with_symbol("instrument?");
    let classes_key = Scm::with_symbol("classes");
    let lv2_sym = Scm::with_symbol("lv2");
    Scm::from_exact_iter(STATE.world.iter_plugins().map(move |p| {
        Scm::EOL
            .acons(is_instrument_key, p.is_instrument())
            .acons(name_key, p.name().as_str())
            .acons(
                id_key,
                Scm::from_exact_iter([Scm::new(lv2_sym), Scm::new(p.uri())].into_iter()),
            )
            .acons(
                classes_key,
                Scm::from_exact_iter(p.classes().map(|c| c.to_scm())),
            )
    }))
}

unsafe extern "C" fn instantiate_plugin(track_id: Scm, plugin_id: Scm) -> Scm {
    let track_id = u32::from_scm(track_id);
    if plugin_id.length() != 2 {
        warn!(
            "Expected plugin-id as pair but got length {}.",
            plugin_id.length()
        );
        return false.to_scm();
    }
    let plugin_ns = String::from_scm(plugin_id.list_ref(0).symbol_to_str().to_scm());
    if plugin_ns != "lv2" {
        warn!("Plugin id space {:?} not recognized.", plugin_ns);
        return false.to_scm();
    }
    let plugin_uri = String::from_scm(plugin_id.list_ref(1));
    let plugin = match STATE.world.plugin_by_uri(&plugin_uri) {
        Some(p) => p,
        None => return false.to_scm(),
    };
    let plugin_instance = match plugin.instantiate(
        STATE.features.clone(),
        STATE.client.as_client().sample_rate() as f64,
    ) {
        Ok(i) => i,
        Err(err) => {
            error!("Failed to instantiate plugin {plugin_uri}: {:?}", err);
            return false.to_scm();
        }
    };
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

unsafe extern "C" fn make_track() -> Scm {
    let id = STATE
        .next_id
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let track = Track {
        id,
        plugin_instances: Vec::with_capacity(16),
        enabled: true,
        volume: 0.5,
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

unsafe extern "C" fn tracks() -> Scm {
    struct TrackInfo {
        id: u32,
        plugin_count: u8,
        volume: f32,
        enabled: bool,
    }
    let mut tracks = Vec::with_capacity(64);
    let tracks = STATE
        .executor
        .execute(move |s| {
            tracks.extend(s.tracks.iter().map(|t| TrackInfo {
                id: t.id,
                plugin_count: t.plugin_instances.len() as u8,
                volume: t.volume,
                enabled: t.enabled,
            }));
            tracks
        })
        .unwrap();
    let id_key = Scm::with_symbol("id");
    let volume_key = Scm::with_symbol("volume");
    let enabled_key = Scm::with_symbol("enabled?");
    let plugin_count_key = Scm::with_symbol("plugin-count");
    Scm::from_exact_iter(tracks.iter().map(|t| {
        Scm::EOL
            .acons(id_key, t.id)
            .acons(enabled_key, t.enabled)
            .acons(volume_key, t.volume)
            .acons(plugin_count_key, t.plugin_count)
    }))
}
