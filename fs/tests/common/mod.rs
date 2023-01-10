use log::{info, LevelFilter};

pub mod block_file;

pub fn setup() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(LevelFilter::Debug)
        .try_init();
}
