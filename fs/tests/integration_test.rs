mod common;

use fs::block_dev::InodeType;

#[test]
fn it_works() {
    let _fs = common::setup();
}

#[test]
fn create_file() {
    let fs = common::setup();
    let root_lock = fs.root();

    let mut root = root_lock.lock();

    for i in 1..100 {
        let dir_lock = root.create(&i.to_string(), InodeType::Directory).unwrap();
        let mut dir = dir_lock.lock();

        for i in 1..100 {
            let file_lock = dir.create(&i.to_string(), InodeType::File).unwrap();
            let mut file = file_lock.lock();
            assert_eq!(file.size(), 0);

            file.resize(i * 500).unwrap();
            assert_eq!(file.size(), i * 500);
        }
    }
}
