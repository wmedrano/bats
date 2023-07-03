use std::{ffi::CStr, sync::Arc};

use crate::{jack_adapter::JackProcessHandler, remote_executor::RemoteExecutor, track::Track};
use flashkick::Scm;
use lazy_static::lazy_static;
use log::{error, info, warn};

/// Register all scheme functions.
pub unsafe fn register_functions() {
    std::thread::spawn(|| {
        // Initialize state in a separate thread to improve
        // responsiveness.
        let _ = &*STATE;
    });
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
        CStr::from_bytes_with_nul(b"track-count\0").unwrap(),
        0,
        0,
        0,
        track_count as _,
    );
}

struct State {
    executor: RemoteExecutor,
    world: livi::World,
    features: Arc<livi::Features>,
    client: jack::AsyncClient<(), JackProcessHandler>,
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
        }
    };
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
            .acons(name_key, p.name())
            .acons(
                id_key,
                Scm::from_exact_iter([Scm::from(lv2_sym), Scm::from(p.uri())].into_iter()),
            )
            .acons(
                classes_key,
                Scm::from_exact_iter(p.classes().map(Scm::from)),
            )
    }))
}

unsafe extern "C" fn instantiate_plugin(track: Scm, plugin_id: Scm) -> Scm {
    let track_idx = Into::<u32>::into(track) as usize;
    if plugin_id.length() != 2 {
        return false.into();
    }
    let plugin_ns: String = plugin_id.list_ref(0).symbol_to_str().into();
    if plugin_ns != "lv2" {
        return false.into();
    }
    let plugin_uri: String = plugin_id.list_ref(1).into();
    let plugin = match STATE.world.plugin_by_uri(&plugin_uri) {
        Some(p) => p,
        None => return false.into(),
    };
    let plugin_instance = match plugin.instantiate(
        STATE.features.clone(),
        STATE.client.as_client().sample_rate() as f64,
    ) {
        Ok(i) => i,
        Err(err) => {
            error!("Failed to instantiate plugin {plugin_uri}: {:?}", err);
            return false.into();
        }
    };
    let did_add = STATE
        .executor
        .execute(move |s| match s.tracks.get_mut(track_idx) {
            None => false,
            Some(t) => {
                t.plugin_instances.push(plugin_instance);
                true
            }
        })
        .unwrap();
    did_add.into()
}

unsafe extern "C" fn make_track() -> Scm {
    let track = Track {
        plugin_instances: Vec::with_capacity(16),
        enabled: true,
        volume: 0.5,
    };
    let track_idx = STATE
        .executor
        .execute(move |s| {
            s.tracks.push(track);
            s.tracks.len() - 1
        })
        .unwrap();
    Scm::from(track_idx as u32)
}

unsafe extern "C" fn delete_track(track_idx: Scm) -> Scm {
    let track_idx: u32 = track_idx.into();
    let track_idx = track_idx as usize;
    let maybe_track = STATE
        .executor
        .execute(move |s| {
            if track_idx < s.tracks.len() {
                Some(s.tracks.remove(track_idx))
            } else {
                None
            }
        })
        .unwrap();
    Scm::from(maybe_track.is_some())
}

unsafe extern "C" fn track_count() -> Scm {
    let count = STATE.executor.execute(|s| s.tracks.len()).unwrap();
    Scm::from(count as u32)
}
