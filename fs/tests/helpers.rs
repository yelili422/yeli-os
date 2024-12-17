use alloc::{format, sync::Arc};
use spin::Mutex;
use std::io::{Read, Seek, SeekFrom, Write};

use fs::{
    block_dev::{BlockDevice, BLOCK_SIZE},
    FileSystem,
};

extern crate alloc;
extern crate std;

pub struct BlockFile(pub Mutex<std::fs::File>);

impl BlockDevice for BlockFile {
    fn read(&self, block_id: u64, buf: &mut [u8])  -> Result<(), String>  {
        let mut file = self.0.lock();
        file.seek(SeekFrom::Start(block_id * BLOCK_SIZE as u64))
            .unwrap();
        assert_eq!(file.read(buf).unwrap(), BLOCK_SIZE);
        Ok(())
    }

    fn write(&self, block_id: u64, buf: &[u8]) -> Result<(), String>  {
        let mut file = self.0.lock();
        file.seek(SeekFrom::Start(block_id * BLOCK_SIZE as u64))
            .unwrap();
        assert_eq!(file.write(buf).unwrap(), BLOCK_SIZE);
        Ok(())
    }
}

pub fn init_test_logger() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();
}

pub fn init_fs() -> Arc<FileSystem> {
    init_test_logger();

    let path = format!("target/fs-{}.img", rand::prelude::random::<u64>());
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .unwrap();
    file.set_len(100 * 1024 * BLOCK_SIZE as u64).unwrap();

    FileSystem::create(
        Arc::new(BlockFile(Mutex::new(file))),
        100 * 1024,
        FileSystem::calc_inodes_num(100 * 1024, 0.1),
    )
    .unwrap()
}
