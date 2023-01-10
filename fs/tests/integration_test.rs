use std::{fs::OpenOptions, sync::Arc};

use fs::FileSystem;
use log::info;
use spin::Mutex;

use crate::common::block_file::BlockFile;

mod common;

#[test]
fn test_run() {
    common::setup();

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("target/fs.img")
        .unwrap();
    file.set_len(4096 * 512).unwrap();

    let fs = FileSystem::create(Arc::new(BlockFile(Mutex::new(file))), 4096, 10).unwrap();
}
