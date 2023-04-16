use std::{
    fs::OpenOptions,
    sync::{Arc, Once},
};

use fs::{block_dev::BLOCK_SIZE, inode::Inode, FileSystem};
use log::LevelFilter;
use spin::Mutex;

use self::block_file::BlockFile;

pub mod block_file;

static INIT: Once = Once::new();

// Hold a global reference of file system for avoiding release.
static mut FS: Option<Arc<FileSystem>> = None;

const FS_PATH: &str = "target/fs.img";

fn init_logger() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(LevelFilter::Debug)
        .try_init();
}

pub fn fs() -> Arc<FileSystem> {
    init_logger();

    INIT.call_once(|| {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(FS_PATH)
            .unwrap();
        file.set_len(100 * 1024 * BLOCK_SIZE as u64).unwrap();

        let fs = FileSystem::create(
            Arc::new(BlockFile(Mutex::new(file))),
            100 * 1024,
            FileSystem::calc_inodes_num(100 * 1024, 0.1),
        )
        .unwrap();

        unsafe { FS = Some(fs.clone()) }
    });

    return unsafe {
        let fs = FS.clone().unwrap();

        let super_block = *fs.sb;
        let fs_root_lock = fs.init(super_block).unwrap();
        let fs_root = fs_root_lock.lock();
        assert_eq!(fs_root.inode_num, 0);

        fs
    };
}

pub fn fs_root() -> Arc<Mutex<Inode>> {
    fs().root()
}
