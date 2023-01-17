use std::{
    fs::OpenOptions,
    sync::{Arc, Once},
};

use fs::{block_cache::BlockCacheBuffer, inode::InodeCacheBuffer, FileSystem};
use log::LevelFilter;
use spin::Mutex;

use self::block_file::BlockFile;

pub mod block_file;

static INIT: Once = Once::new();

const FS_PATH: &str = "target/fs.img";

pub fn setup() -> Arc<FileSystem> {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(LevelFilter::Debug)
        .try_init();

    INIT.call_once(|| {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(FS_PATH)
            .unwrap();
        file.set_len(4096 * 512).unwrap();

        FileSystem::create(
            Arc::new(BlockFile(Mutex::new(file))),
            Arc::new(Mutex::new(BlockCacheBuffer::new())),
            Arc::new(Mutex::new(InodeCacheBuffer::new())),
            4096,
            10,
        )
        .unwrap();
    });

    FileSystem::open(
        Arc::new(BlockFile(Mutex::new(
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(FS_PATH)
                .unwrap(),
        ))),
        Arc::new(Mutex::new(BlockCacheBuffer::new())),
        Arc::new(Mutex::new(InodeCacheBuffer::new())),
    )
    .unwrap()
}
