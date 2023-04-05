use fs::block_dev::{BlockDevice, BLOCK_SIZE};
use spin::Mutex;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};

pub struct BlockFile(pub Mutex<File>);

impl BlockDevice for BlockFile {
    fn read(&self, block_id: u64, buf: &mut [u8]) {
        let mut file = self.0.lock();
        file.seek(SeekFrom::Start(block_id * BLOCK_SIZE as u64))
            .unwrap();
        assert_eq!(file.read(buf).unwrap(), BLOCK_SIZE);
    }

    fn write(&self, block_id: u64, buf: &[u8]) {
        let mut file = self.0.lock();
        file.seek(SeekFrom::Start(block_id * BLOCK_SIZE as u64))
            .unwrap();
        assert_eq!(file.write(buf).unwrap(), BLOCK_SIZE);
    }
}
