use log::info;

mod bats;
mod jack_adapter;
mod remote_executor;
mod scheme_adapter;
mod track;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .init();

    let args = std::env::args();
    info!("{:?}", args);

    flashkick::boot_with_shell(args, inner_main);
}

extern "C" fn inner_main(_argc: i32, _argv: *mut *mut i8) {
    unsafe { scheme_adapter::init_bats() };
}