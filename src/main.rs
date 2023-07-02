use std::{
    ffi::{c_char, c_void, CString},
    sync::Arc,
};

use guile_3_sys::{
    scm_boot_guile, scm_c_define_gsubr, scm_from_int8, scm_from_uint32, scm_from_utf8_stringn,
    scm_shell, scm_to_uint32, scm_to_utf8_stringn, SCM,
};
use jack_adapter::JackProcessHandler;
use lazy_static::lazy_static;
use log::{error, info, warn};
use remote_executor::RemoteExecutor;
use track::Track;

mod jack_adapter;
mod remote_executor;
mod simian;
mod track;

struct State {
    executor: RemoteExecutor,
    world: livi::World,
    features: Arc<livi::Features>,
    client: jack::AsyncClient<(), JackProcessHandler>,
}

lazy_static! {
    static ref STATE: State = {
        let world_and_features = std::thread::spawn(new_world_and_features);
        let (client, status) =
            jack::Client::new("simian-sonic", jack::ClientOptions::NO_START_SERVER).unwrap();
        let sample_rate = client.sample_rate() as f64;
        info!(
            "Created {}(sample_rate={sample_rate}) with status {status:?}.",
            client.name()
        );

        let (world, features) = world_and_features.join().unwrap();
        let mut process_handler =
            jack_adapter::JackProcessHandler::new(&client, &features).unwrap();
        let executor = process_handler.simian.reset_remote_executor(1);
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

unsafe extern "C" fn plugin_count() -> SCM {
    scm_from_uint32(STATE.world.iter_plugins().count() as u32)
}

unsafe extern "C" fn plugin_uri(idx: SCM) -> SCM {
    let idx = scm_to_uint32(idx);
    match STATE.world.iter_plugins().nth(idx as usize) {
        None => scm_from_utf8_stringn(std::ptr::null(), 0),
        Some(p) => {
            let uri = p.uri();
            scm_from_utf8_stringn(uri.as_str().as_ptr() as _, uri.len() as _)
        }
    }
}

unsafe extern "C" fn plugin_name(idx: SCM) -> SCM {
    let idx = scm_to_uint32(idx);
    match STATE.world.iter_plugins().nth(idx as usize) {
        None => scm_from_utf8_stringn(std::ptr::null(), 0),
        Some(p) => {
            let name = p.name();
            scm_from_utf8_stringn(name.as_str().as_ptr() as _, name.len() as _)
        }
    }
}

unsafe extern "C" fn instantiate_plugin(track: SCM, plugin_uri: SCM) -> SCM {
    let track_idx = scm_to_uint32(track) as usize;
    let mut plugin_uri_len = 0;
    let plugin_uri_ptr = scm_to_utf8_stringn(plugin_uri, &mut plugin_uri_len) as *const u8;
    let plugin_uri_bytes = std::slice::from_raw_parts(plugin_uri_ptr, plugin_uri_len as _);
    let plugin_uri = std::str::from_utf8(plugin_uri_bytes).unwrap();
    let plugin = STATE.world.plugin_by_uri(plugin_uri);
    let ret = match plugin {
        None => 0,
        Some(plugin) => {
            let maybe_plugin_instance = plugin.instantiate(
                STATE.features.clone(),
                STATE.client.as_client().sample_rate() as f64,
            );
            match maybe_plugin_instance {
                Ok(plugin_instance) => STATE
                    .executor
                    .execute(move |s| match s.tracks.get_mut(track_idx) {
                        None => 0,
                        Some(t) => {
                            t.plugin_instances.push(plugin_instance);
                            1
                        }
                    })
                    .unwrap(),
                Err(err) => {
                    error!("Failed to instantiate plugin {}: {:?}", plugin_uri, err);
                    0
                }
            }
        }
    };
    scm_from_int8(ret)
}

unsafe extern "C" fn make_track() -> SCM {
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
    scm_from_uint32(track_idx as _)
}

unsafe extern "C" fn delete_track(track_idx: SCM) -> SCM {
    let track_idx = scm_to_uint32(track_idx) as usize;
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
    scm_from_int8(if maybe_track.is_some() { 1 } else { 0 })
}

unsafe extern "C" fn track_count() -> SCM {
    let count = STATE.executor.execute(|s| s.tracks.len()).unwrap();
    scm_from_uint32(count as _)
}

unsafe extern "C" fn inner_main(_: *mut c_void, argc: i32, argv: *mut *mut i8) {
    scm_c_define_gsubr(b"plugin-count\0".as_ptr() as _, 0, 0, 0, plugin_count as _);
    scm_c_define_gsubr(b"plugin-uri\0".as_ptr() as _, 1, 0, 0, plugin_uri as _);
    scm_c_define_gsubr(b"plugin-name\0".as_ptr() as _, 1, 0, 0, plugin_name as _);
    scm_c_define_gsubr(b"make-track\0".as_ptr() as _, 0, 0, 0, make_track as _);
    scm_c_define_gsubr(
        b"instantiate-plugin\0".as_ptr() as _,
        2,
        0,
        0,
        instantiate_plugin as _,
    );
    scm_c_define_gsubr(b"delete-track".as_ptr() as _, 1, 0, 0, delete_track as _);
    scm_c_define_gsubr(b"track-count".as_ptr() as _, 0, 0, 0, track_count as _);
    scm_shell(argc, argv);
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    info!("{:?}", std::env::args());
    std::thread::spawn(|| {
        let _ = &*STATE;
    });

    info!("Initializing scheme.");

    info!("Running scheme.");
    unsafe {
        let argv: Vec<CString> = std::env::args()
            .map(CString::new)
            .map(Result::unwrap)
            .collect();
        let args: Vec<*const c_char> = argv.into_iter().map(|arg| arg.as_ptr()).collect();
        scm_boot_guile(
            args.len() as _,
            args.as_ptr() as _,
            Some(inner_main),
            std::ptr::null_mut(),
        );
    };
}

fn new_world_and_features() -> (livi::World, Arc<livi::Features>) {
    let world = livi::World::new();
    let features = world.build_features(livi::FeaturesBuilder::default());
    (world, features)
}
