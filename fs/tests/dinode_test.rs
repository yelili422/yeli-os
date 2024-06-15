use fs::block_dev::{InodeType, BLOCK_SIZE};

mod common;

#[test]
fn block_id_test() {
    let root_lock = common::fs_root();
    let mut root = root_lock.lock();

    let file_lock = root.create("get_dinode_id", InodeType::File).unwrap();
    let mut file = file_lock.lock();
    file.resize(155 * BLOCK_SIZE).unwrap();
    let buffer = [255u8; BLOCK_SIZE];

    for i in 0..155 {
        file.write_data(i * BLOCK_SIZE, &buffer);
    }
}
