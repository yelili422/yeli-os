mod common;

use fs::block_dev::InodeType;
use log::info;

#[test]
fn it_works() {
    let _fs = common::setup();
}

#[test]
fn create_file_normal() {
    let fs = common::setup();

    let root_lock = fs.root();
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
    let fs = common::setup();

    let root_lock = fs.root();
    let mut root = root_lock.lock();

    let file_lock = root.create("a_large_file", InodeType::File).unwrap();
    let mut file = file_lock.lock();
    assert_eq!(file.size(), 0);

    file.resize(16 * 1024 * 1024).unwrap();
    assert_eq!(file.size(), 16 * 1024 * 1024);
}

#[test]
fn create_amounts_of_directories() {
    let fs = common::setup();

    let root_lock = fs.root();
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
