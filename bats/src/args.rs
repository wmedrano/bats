use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// If true, then ports will automatically be connected.
    #[arg(long, default_value_t = true)]
    pub auto_connect_ports: bool,

    /// The amount of logging to perform. The values are OFF, ERROR, WARN, INFO, DEBUG, and TRACE.
    #[arg(long, default_value_t = log::LevelFilter::Info)]
    pub log_level: log::LevelFilter,

    /// Load up an initial plugin by default.
    #[arg(long, default_value_t = true)]
    pub load_initial_plugin: bool,
}
