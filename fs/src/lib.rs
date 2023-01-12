#![no_std]
#![feature(drain_filter)]

extern crate alloc;

use alloc::{string::String, sync::Arc};
use block_dev::{
    BitmapBlock, BlockDevice, BlockId, DInode, InBlockOffset, InodeId, InodeType, SuperBlock,
    BLOCK_SIZE, DINODE_SIZE, INODES_PER_BLOCK,
};
use buffer_cache::{block_cache, flush};
use core::mem::size_of;
use inode::Inode;
use log::info;
use spin::Mutex;

use crate::block_dev::{MAX_BLOCKS_ONE_INODE, MAX_SIZE_ONE_INODE};

pub mod block_dev;
mod buffer_cache;
mod file;
mod inode;

pub struct FileSystem {
    dev:         Arc<dyn BlockDevice>,
    /// A copy of super block in memory.
    /// We can't edit the data in super block on disk during the
    /// file system running except when it creating. Therefor,
    /// we can use it safely.
    super_block: Arc<SuperBlock>,
    lock:        Mutex<()>,
}

impl FileSystem {
    pub fn create(
        dev: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_blocks: u32,
    ) -> Result<Arc<Self>, FileSystemCreateError> {
        info!("fs: block size: {}", BLOCK_SIZE);
        info!("fs: inode size: {}", size_of::<DInode>());
        info!("fs: max blocks of one inode: {}", MAX_BLOCKS_ONE_INODE);
        info!("fs: max size of one inode: {}", MAX_SIZE_ONE_INODE);

        let inode_bmap_blocks = inode_blocks * BLOCK_SIZE as u32 / size_of::<DInode>() as u32 + 1;
        let inode_area = inode_bmap_blocks + inode_blocks;

        let data_area = total_blocks - 2 - inode_area; // bitmap + data blocks
        let data_bmap_blocks = (data_area / (1 + 8 * BLOCK_SIZE as u32)) as u32 + 1;
        let data_blocks = data_area - data_bmap_blocks;

        let super_block_start = 1;
        let inode_bmap_start = 2;
        let inode_start = 3;
        let data_bmap_start = inode_start + inode_blocks;
        let data_start = data_bmap_start + data_bmap_blocks;

        // Clear all non-data blocks.
        for i in 0..data_start {
            block_cache(i, dev.clone())
                .lock()
                .write(0, |data_block: &mut [u8; BLOCK_SIZE]| {
                    for b in data_block.iter_mut() {
                        *b = 0;
                    }
                })
        }

        // Initialize the super block.
        block_cache(super_block_start, dev.clone()).lock().write(
            0,
            |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks,
                    data_blocks,
                    inode_blocks,
                    inode_bmap_start,
                    inode_start,
                    data_bmap_start,
                    data_start,
                );
            },
        );

        flush();

        let fs = FileSystem::open(dev).expect("Create file system failed.");

        // Create the root inode and initialize it.
        let root_inode = fs
            .allocate_inode(InodeType::Directory)
            .expect("Create root inode failed.");
        assert_eq!(root_inode.inode_num, 0);

        flush();

        Ok(fs)
    }

    pub fn open(dev: Arc<dyn BlockDevice>) -> Result<Arc<Self>, FileSystemInvalid> {
        block_cache(1, dev.clone())
            .lock()
            .read(0, |super_block: &SuperBlock| {
                if super_block.is_valid() {
                    Ok(Arc::new(Self {
                        dev:         dev.clone(),
                        super_block: Arc::new(super_block.clone()),
                        lock:        Mutex::new(()),
                    }))
                } else {
                    Err(FileSystemInvalid())
                }
            })
    }

    /// Allocates a new empty inode from current file system.
    pub fn allocate_inode(self: &Arc<Self>, type_: InodeType) -> Option<Arc<Inode>> {
        // Acquire the global lock to synchronize inode allocations.
        let _lock = self.lock.lock();

        if let Some(inum) = block_cache(self.super_block.inode_bmap_start, self.dev.clone())
            .lock()
            .write(0, |inode_bmap: &mut BitmapBlock| inode_bmap.allocate())
        {
            let inode = Inode::from_inum(inum as InodeId, self.dev.clone(), self.clone());
            inode.write_dinode(|dinode| dinode.initialize(type_));

            Some(inode)
        } else {
            None
        }
    }

    /// Allocates a free space in data area.
    pub fn allocate(self: &Arc<Self>) -> Option<BlockId> {
        let _lock = self.lock.lock();

        if let Some(block_offset) = block_cache(self.super_block.data_bmap_start, self.dev.clone())
            .lock()
            .write(0, |data_bmap: &mut BitmapBlock| data_bmap.allocate())
        {
            Some(self.super_block.data_start + block_offset as BlockId)
        } else {
            None
        }
    }

    pub fn root(self: &Arc<Self>) -> Arc<Inode> {
        Inode::from_inum(0, self.dev.clone(), self.clone())
    }

    /// Gets block id and offset-in-block by inode-num.
    pub fn inode_pos(&self, inum: InodeId) -> (BlockId, InBlockOffset) {
        let block_id = inum / INODES_PER_BLOCK as u32 + self.super_block.inode_start;
        let offset = (inum % INODES_PER_BLOCK as u32) * DINODE_SIZE as u32;
        (block_id, offset)
    }
}

#[derive(Debug)]
pub struct FileSystemCreateError();

#[derive(Debug)]
pub struct FileSystemInvalid();

#[derive(Debug)]
pub enum FileSystemAllocationError {
    Exhausted(usize),
    InodeExhausted,
    AlreadyExist(String, InodeType),
    TooLarge(usize),
}
