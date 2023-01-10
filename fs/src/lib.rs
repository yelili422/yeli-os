#![no_std]
#![feature(drain_filter)]

extern crate alloc;

use alloc::sync::Arc;
use block_dev::{BitmapBlock, BlockDevice, DInode, InodeType, SuperBlock, BLOCK_SIZE};
use buffer_cache::{block_cache, flush};
use core::mem::size_of;
use inode::Inode;
use log::info;
use spin::Mutex;

use crate::block_dev::MAX_BLOCKS_ONE_INODE;

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
}

impl FileSystem {
    pub fn create(
        dev: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_blocks: u32,
    ) -> Result<Arc<Mutex<Self>>, FileSystemCreateError> {
        info!("fs: block size: {}", BLOCK_SIZE);
        info!("fs: inode size: {}", size_of::<DInode>());
        info!("fs: max blocks of one inode: {}", MAX_BLOCKS_ONE_INODE);

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
            block_cache(i, Arc::clone(&dev)).lock().write_to(
                0,
                |data_block: &mut [u8; BLOCK_SIZE]| {
                    for b in data_block.iter_mut() {
                        *b = 0;
                    }
                },
            )
        }

        // Initialize the super block.
        block_cache(super_block_start, Arc::clone(&dev))
            .lock()
            .write_to(0, |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks,
                    data_blocks,
                    inode_blocks,
                    inode_bmap_start,
                    inode_start,
                    data_bmap_start,
                    data_start,
                );
            });

        flush();

        let (root_block_id, root_offset) = block_cache(super_block_start, Arc::clone(&dev))
            .lock()
            .read_from(0, |super_block: &SuperBlock| super_block.inode_pos(0));

        // Create the root inode.
        block_cache(root_block_id, Arc::clone(&dev))
            .lock()
            .write_to(root_offset, |inode: &mut DInode| {
                inode.initialize(InodeType::Directory);
            });

        flush();

        Ok(FileSystem::open(dev).expect("Create file system failed."))
    }

    pub fn open(dev: Arc<dyn BlockDevice>) -> Result<Arc<Mutex<Self>>, FileSystemInvalid> {
        block_cache(1, Arc::clone(&dev))
            .lock()
            .read_from(0, |super_block: &SuperBlock| {
                if super_block.is_valid() {
                    Ok(Arc::new(Mutex::new(Self {
                        dev:         Arc::clone(&dev),
                        super_block: Arc::new(super_block.clone()),
                    })))
                } else {
                    Err(FileSystemInvalid())
                }
            })
    }

    /// Allocates a new inode from current file system.
    pub fn allocate(&self) -> Option<Arc<Inode>> {
        if let Some(inum) = block_cache(self.super_block.inode_bmap_start, Arc::clone(&self.dev))
            .lock()
            .write_to(0, |bmap: &mut BitmapBlock| bmap.allocate())
        {
            Some(Arc::new(Inode::from_inum(
                inum,
                Arc::clone(&self.dev),
                Arc::clone(&self.super_block),
            )))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct FileSystemCreateError();

#[derive(Debug)]
pub struct FileSystemInvalid();
