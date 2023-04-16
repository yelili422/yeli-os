mod common;

use std::{env, fs::OpenOptions, io::Read};

use fs::block_dev::{InodeType, BLOCK_SIZE};
use log::info;

#[test]
fn it_works() {
    let _ = common::fs();
}

#[test]
fn allocate_block_test() {
    let fs = common::fs();
    let data_blocks_num = fs.sb.data_blocks;
    for i in 0..data_blocks_num {
        assert_eq!(fs.allocate_block(), Some(fs.sb.data_start + i));
    }
}

#[test]
fn create_file_normal() {
    let root_lock = common::fs_root();
    let mut root = root_lock.lock();

    for i in 1..100 {
        let dir_lock = root.create(&i.to_string(), InodeType::Directory).unwrap();
        let mut dir = dir_lock.lock();

        for i in 1..10 {
            let file_lock = dir.create(&i.to_string(), InodeType::File).unwrap();
            let mut file = file_lock.lock();
            assert_eq!(file.size(), 0);

            file.resize(10).unwrap();
            assert_eq!(file.size(), 10);
        }
    }
}

#[test]
fn create_single_large_file() {
    let root_lock = common::fs_root();
    let mut root = root_lock.lock();

    let file_lock = root.create("a_large_file", InodeType::File).unwrap();
    let mut file = file_lock.lock();
    assert_eq!(file.size(), 0);

    file.resize(16 * 1024 * 1024).unwrap();
    assert_eq!(file.size(), 16 * 1024 * 1024);
}

#[test]
fn create_amounts_of_directories() {
    let root_lock = common::fs_root();
    let mut root = root_lock.lock();

    let dir_lock = root
        .create("amounts_of_directories", InodeType::Directory)
        .unwrap();
    let mut dir = dir_lock.lock();

    for i in 1..500 {
        info!("creating the {} directory", i);
        let d_lock = dir.create(&i.to_string(), InodeType::Directory).unwrap();
        let d = d_lock.lock();

        assert_eq!(d.type_(), InodeType::Directory);
    }
}

#[test]
fn read_and_write() {
    let args: Vec<_> = env::args().collect();
    let file_path = &args[0];

    let mut src_file = OpenOptions::new().read(true).open(file_path).unwrap();
    let len = src_file.metadata().unwrap().len();

    let root_lock = common::fs_root();
    let mut root = root_lock.lock();

    let dst_file_lock = root.create("read_and_write", InodeType::File).unwrap();
    let mut dst_file = dst_file_lock.lock();

    dst_file.resize(len as usize).unwrap();

    let mut buffer = [0u8; BLOCK_SIZE];
    let mut read_count = 0;
    loop {
        let offset = src_file.read(&mut buffer).unwrap();
        if offset == 0 {
            break;
        }

        dst_file.write_data(read_count, &buffer);
        read_count += offset;
    }
}
