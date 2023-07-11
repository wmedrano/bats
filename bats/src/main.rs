use hyper::Method;
use jsonrpsee_server::{types::ErrorObject, AllowHosts, RpcModule, ServerBuilder, ServerHandle};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

mod bats;
mod jack_adapter;
mod remote_executor;
mod state;
mod track;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .unwrap();
    let addr = "127.0.0.1:8000".parse::<SocketAddr>().unwrap();
    let state = state::State::new().unwrap();
    let _server_handle = run_server(addr, state).await;
    std::thread::park();
}

async fn run_server(addr: SocketAddr, state: state::State) -> ServerHandle {
    // Add a CORS middleware for handling HTTP requests.
    // This middleware does affect the response, including appropriate
    // headers to satisfy CORS. Because any origins are allowed, the
    // "Access-Control-Allow-Origin: *" header is appended to the response.
    let cors = CorsLayer::new()
        // Allow `POST` when accessing the resource
        .allow_methods([Method::POST])
        // Allow requests from any origin
        .allow_origin(Any)
        .allow_headers([hyper::header::CONTENT_TYPE]);
    let middleware = tower::ServiceBuilder::new().layer(cors);

    // The RPC exposes the access control for filtering and the middleware for
    // modifying requests / responses. These features are independent of one another
    // and can also be used separately.
    // In this example, we use both features.
    let server = ServerBuilder::new()
        .set_host_filtering(AllowHosts::Any)
        .set_middleware(middleware)
        .build(addr)
        .await
        .unwrap();

    let mut module = RpcModule::new(state);
    module
        .register_method("settings", move |_params, state| Some(state.settings()))
        .unwrap();
    module
        .register_method("plugins", move |_params, state| state.plugins())
        .unwrap();
    module
        .register_method("make_track", move |_params, state| Some(state.make_track()))
        .unwrap();
    module
        .register_method("delete_track", move |params, state| {
            state
                .delete_track(params.parse()?)
                .map(Some)
                .map_err(error_to_obj)
        })
        .unwrap();
    module
        .register_method("tracks", move |_params, state| state.tracks())
        .unwrap();
    module
        .register_method("make_plugin_instance", move |params, state| {
            state
                .make_plugin_instance(params.parse()?)
                .map(Some)
                .map_err(error_to_obj)
        })
        .unwrap();
    module
        .register_method("delete_plugin_instance", move |params, state| {
            state
                .delete_plugin_instance(params.parse()?)
                .map(Some)
                .map_err(error_to_obj)
        })
        .unwrap();
    module
        .register_method("plugin_instances", move |_params, state| {
            state.plugin_instances()
        })
        .unwrap();

    server.start(module).unwrap()
}

fn error_to_obj<E: std::fmt::Display>(e: E) -> ErrorObject<'static> {
    ErrorObject::owned(500, e.to_string(), Some(()))
}
