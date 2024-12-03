use std::io::Read;

use fs::block_dev::{self, InodeType, BLOCK_SIZE, CAPACITY_PER_INODE};
use log::debug;

extern crate alloc;
extern crate std;

mod helpers;

#[test]
fn test_it_works() {
    let fs = helpers::init_fs();
    let root_lock = fs.root();
    let root = root_lock.lock();

    assert_eq!(root.inode_num, 0);
    assert_eq!(root.size(), 0);
    assert_eq!(root.type_, InodeType::Directory);
}

#[test]
fn test_allocate_block() {
    let fs = helpers::init_fs();
    debug!("fs: max blocks num: {}", fs.max_blocks_num());
    for i in 0..fs.max_blocks_num() {
        let block_id = fs.allocate_data_block();
        assert_eq!(block_id, Some(fs.sb.data_start + i), "Failed to allocate the {}th block", i);
    }
    assert_eq!(fs.allocate_data_block(), None, "Exceeding the max blocks num.");
}

#[test]
fn test_nested_dir() {
    let fs = helpers::init_fs();
    let root_lock = fs.root();
    let mut root = root_lock.lock();

    for i in 1..10 {
        let dir_lock = fs
            .create_inode(&mut root, &i.to_string(), InodeType::Directory)
            .unwrap();
        let mut dir = dir_lock.lock();

        for j in 1..10 {
            let inner_dir_lock = fs
                .create_inode(&mut dir, &j.to_string(), InodeType::Directory)
                .unwrap();
            let mut inner_dir = inner_dir_lock.lock();

            for k in 1..10 {
                let file_lock = fs
                    .create_inode(&mut inner_dir, &k.to_string(), InodeType::File)
                    .unwrap();
                let mut file = file_lock.lock();
                assert_eq!(file.size(), 0);

                fs.resize_inode(&mut file, 10).unwrap();
                assert_eq!(file.size(), 10);

                fs.write_inode(&file, 0, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
                let mut buffer = [0u8; 10];
                fs.read_inode(&file, 0, &mut buffer);
                assert_eq!(buffer, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
            }
        }
    }
}

#[test]
fn test_single_large_file() {
    let fs = helpers::init_fs();
    let root_lock = fs.root();
    let mut root = root_lock.lock();

    let file_lock = fs
        .create_inode(&mut root, "a_large_file", InodeType::File)
        .unwrap();
    let mut file = file_lock.lock();
    assert_eq!(file.size(), 0);

    fs.resize_inode(&mut file, CAPACITY_PER_INODE).unwrap();
    assert_eq!(file.size(), CAPACITY_PER_INODE);

    let res = fs.resize_inode(&mut file, CAPACITY_PER_INODE + 1);
    assert!(res.is_err());
}

#[test]
#[ignore = "This test will take a very long time to run"]
fn test_amounts_of_directories() {
    let fs = helpers::init_fs();
    let root_lock = fs.root();
    let mut root = root_lock.lock();

    let dir_lock = fs
        .create_inode(&mut root, "amounts_of_directories", InodeType::Directory)
        .unwrap();
    let mut dir = dir_lock.lock();

    for i in 0..block_dev::MAX_DIRENTS_PER_INODE {
        let d_lock = fs
            .create_inode(&mut dir, &i.to_string(), InodeType::Directory)
            .unwrap();
        let d = d_lock.lock();

        assert_eq!(d.type_, InodeType::Directory);
    }
}

#[test]
fn test_read_write() {
    let args: alloc::vec::Vec<_> = std::env::args().collect();
    let file_path = &args[0];

    let mut src_file = std::fs::OpenOptions::new()
        .read(true)
        .open(file_path)
        .unwrap();

    let fs = helpers::init_fs();
    let root_lock = fs.root();
    let mut root = root_lock.lock();

    let dst_file_lock = fs
        .create_inode(&mut root, "read_and_write", InodeType::File)
        .unwrap();
    let mut dst_file = dst_file_lock.lock();

    fs.resize_inode(&mut dst_file, block_dev::CAPACITY_PER_INODE)
        .unwrap();

    let mut buffer = [0u8; BLOCK_SIZE];
    let mut read_count = 0;
    loop {
        let offset = Read::read(&mut src_file, &mut buffer).unwrap();
        if offset == 0 {
            break;
        }

        fs.write_inode(&dst_file, read_count, &buffer);
        read_count += offset;

        if read_count >= fs::block_dev::CAPACITY_PER_INODE {
            break;
        }
    }
}
