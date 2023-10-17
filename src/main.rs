use log::info;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .unwrap();
    info!(
        "Current Dir: {:?}, Args: {:?}",
        std::env::current_dir().unwrap(),
        std::env::args().collect::<Vec<_>>()
    );
}
