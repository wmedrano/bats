use hyper::Method;
use jsonrpsee_server::{AllowHosts, RpcModule, ServerBuilder, ServerHandle};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .unwrap();
    let addr = "127.0.0.1:8000".parse::<SocketAddr>().unwrap();
    let _server_handle = run_server(addr).await;
    std::thread::park();
}

async fn run_server(addr: SocketAddr) -> ServerHandle {
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

    let mut module = RpcModule::new(());
    module
        .register_method("say_hello", |_, _| {
            println!("say_hello method called!");
            "Hello there!!"
        })
        .unwrap();

    server.start(module).unwrap()
}
