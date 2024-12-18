use alloc::{sync::Arc, vec, vec::Vec};

use fs::{block_dev::BLOCK_SIZE, inode::Inode, FileSystem};
use spin::Mutex;

use crate::{
    drivers::virtio::virtio_blk::VirtIOBlock, mem::VIRTIO_MMIO_BASE, println,
    sync::once_cell::OnceCell,
};

pub fn init() {
    ROOT_FS
        .init(|| {
            let dev =
                VirtIOBlock::init(VIRTIO_MMIO_BASE).expect("failed to init virtio block device");
            FileSystem::open(dev, true).expect("failed to open file system")
        })
        .unwrap();
}

pub static ROOT_FS: OnceCell<Arc<FileSystem>> = OnceCell::new();

pub fn load_initcode() -> Vec<u8> {
    let fs = ROOT_FS.get().expect("File system not initialized");
    let bin_inode = fs
        .get_inode_from_path("/bin/init", &fs.root())
        .expect("failed to open file");
    InodeFile::open(bin_inode).read_file()
}

pub trait File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()>;

    fn write(&mut self, buf: &[u8]) -> Result<usize, ()>;

    fn read_file(&mut self) -> Vec<u8> {
        let mut buf = vec![0u8; BLOCK_SIZE];
        let mut data = Vec::new();

        loop {
            match self.read(&mut buf) {
                Ok(size) => {
                    data.extend_from_slice(&buf[..size]);
                    if size != buf.len() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        data
    }
}

pub struct InodeFile {
    inode:  Arc<Mutex<Inode>>,
    offset: usize,
}

impl InodeFile {
    pub fn open(inode: Arc<Mutex<Inode>>) -> Self {
        Self { inode, offset: 0 }
    }
}

impl File for InodeFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        let inode = self.inode.lock();
        match inode.get_fs() {
            Some(fs) => {
                let size = fs.read_inode(&inode, self.offset, buf);
                self.offset += size;
                Ok(size)
            }
            None => Err(()),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, ()> {
        let inode = self.inode.lock();
        match inode.get_fs() {
            Some(fs) => {
                let size = fs.write_inode(&inode, self.offset, buf);
                self.offset += size;
                Ok(size)
            }
            None => Err(()),
        }
    }
}
