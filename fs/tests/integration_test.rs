mod common;

use fs::block_dev::InodeType;

#[test]
fn test_create_file() {
    let fs = common::setup();
    let root = fs.root();

    for i in 1..10 {
        let f = root.allocate(&i.to_string(), InodeType::File).unwrap();
        assert_eq!(f.size(), 0);

        f.resize(i * 500).unwrap();
        assert_eq!(f.size(), i * 500);
    }
}
