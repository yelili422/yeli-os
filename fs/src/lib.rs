#![no_std]
#![feature(generic_nonzero)]

extern crate alloc;

use alloc::{string::String, sync::Arc};
use block_cache::BlockCacheBuffer;
use block_dev::{
    BitmapBlock, BlockDevice, BlockId, InodeId, InodeType, SuperBlock, BLOCK_SIZE, DINODE_SIZE,
    INODES_PER_BLOCK,
};
use core::mem::size_of;
use inode::{Inode, InodeCacheBuffer, InodeNotExists};
use log::{debug, warn};
use spin::Mutex;

use crate::block_dev::{MAX_BLOCKS_PER_INODE, CAPACITY_PER_INODE};

pub mod block_cache;
pub mod block_dev;
mod file;
pub mod inode;

/// The location of the super block.
pub const SUPER_BLOCK_LOC: u64 = 1;

pub struct FileSystem {
    dev:         Arc<dyn BlockDevice>,
    // A copy of super block in memory.
    // We can't edit the data in super block on disk during the
    // file system running except when it creating. Therefor,
    // we can use it safely.
    pub sb:      Arc<SuperBlock>,
    // Synchronize access to disk blocks to ensure that only one
    // copy of a block in memory and that only one kernel thread
    // at a time use that copy.
    block_cache: Arc<Mutex<BlockCacheBuffer>>,
    // This lock protects the invariant that an inode is present in the
    // cache at most once.
    inode_cache: Arc<Mutex<InodeCacheBuffer>>,
}

impl FileSystem {
    pub fn calc_inodes_num(total_blocks: u64, factor: f64) -> u64 {
        (total_blocks as f64 * factor) as u64
    }

    pub fn create(
        dev: Arc<dyn BlockDevice>,
        total_blocks_num: u64,
        inode_blocks_num: u64,
    ) -> Result<Arc<Self>, FileSystemInitError> {
        debug!("fs: block size: {} Bytes", BLOCK_SIZE);
        debug!("fs: inode size: {} Bytes", DINODE_SIZE);
        assert_eq!(
            DINODE_SIZE,
            BLOCK_SIZE / INODES_PER_BLOCK,
            "The size of the inode needs to be adapted to the `block_size`"
        );

        debug!("fs: max blocks of one inode: {}", MAX_BLOCKS_PER_INODE);
        debug!(
            "fs: max data size of one inode: {} Bytes({} MBytes)",
            CAPACITY_PER_INODE,
            CAPACITY_PER_INODE / 1024 / 1024
        );

        let super_blocks_num = 1;
        let logging_blocks_num = 1;

        let inode_bmap_blocks_num = inode_blocks_num / (size_of::<BitmapBlock>() as u64) + 1;
        let inode_area = inode_bmap_blocks_num + inode_blocks_num;

        debug!("fs: total blocks: {}", total_blocks_num);
        debug!(
            "fs: inode area: inode bitmap({}) + inode blocks({})",
            inode_bmap_blocks_num, inode_blocks_num
        );

        let data_area = total_blocks_num - super_blocks_num - logging_blocks_num - inode_area; // bitmap + data blocks
        let data_bmap_blocks = (data_area / (1 + 8 * BLOCK_SIZE as u64)) + 1;
        let data_blocks_num = data_area - data_bmap_blocks;

        assert!(
            total_blocks_num > SUPER_BLOCK_LOC + super_blocks_num + logging_blocks_num + inode_area,
            "No more space for data blocks."
        );

        debug!(
            "fs: data area: data bitmap({}) + data blocks({})",
            data_bmap_blocks, data_blocks_num
        );

        let inode_bmap_start = SUPER_BLOCK_LOC + super_blocks_num;
        let inode_start = inode_bmap_start + inode_bmap_blocks_num;
        let data_bmap_start = inode_start + inode_blocks_num;
        let data_start = data_bmap_start + data_bmap_blocks;

        let sb = SuperBlock::new(
            total_blocks_num,
            inode_bmap_start,
            inode_start,
            inode_blocks_num,
            data_bmap_start,
            data_start,
            data_blocks_num,
        );
        debug!("fs: init fs with super block: {:#?}", sb);
        let root_inode = Self::init_fs(dev.clone(), sb).unwrap();
        assert_eq!(root_inode.lock().inode_num, 0);

        Ok(FileSystem::open(dev, true).expect("Failed to create file system."))
    }

    pub fn open(dev: Arc<dyn BlockDevice>, validate: bool) -> Result<Arc<Self>, FileSystemInvalid> {
        let block_cache = Arc::new(Mutex::new(BlockCacheBuffer::new()));
        let inode_cache = Arc::new(Mutex::new(InodeCacheBuffer::new()));

        let mut lock = block_cache.lock();
        lock.get(SUPER_BLOCK_LOC, dev.clone())
            .lock()
            .read(0, |super_block: &SuperBlock| {
                if super_block.is_valid() || !validate {
                    Ok(Arc::new(Self {
                        dev:         dev.clone(),
                        sb:          Arc::new(super_block.clone()),
                        block_cache: block_cache.clone(),
                        inode_cache: inode_cache.clone(),
                    }))
                } else {
                    Err(FileSystemInvalid())
                }
            })
    }

    pub fn init(self: &Arc<Self>, sb: SuperBlock) -> Result<(), FileSystemInitError> {
        let _ = FileSystem::init_fs(self.dev.clone(), sb)?;
        Ok(())
    }

    /// Initialize the file system.
    pub fn init_fs(
        dev: Arc<dyn BlockDevice>,
        sb: SuperBlock,
    ) -> Result<Arc<Mutex<Inode>>, FileSystemInitError> {
        let block_cache = Arc::new(Mutex::new(BlockCacheBuffer::new()));

        // Clear all non-data blocks.
        for i in 0..sb.data_start {
            block_cache.lock().get(i, dev.clone()).lock().write(
                0,
                |data_block: &mut [u8; BLOCK_SIZE]| {
                    for b in data_block.iter_mut() {
                        *b = 0;
                    }
                },
            )
        }

        // Initialize the super block.
        block_cache
            .lock()
            .get(SUPER_BLOCK_LOC, dev.clone())
            .lock()
            .write(0, |super_block: &mut SuperBlock| {
                *super_block = sb;
            });
        block_cache.lock().flush();

        block_cache
            .lock()
            .get(SUPER_BLOCK_LOC, dev.clone())
            .lock()
            .read(0, |sb_in_disk: &SuperBlock| {
                assert_eq!(*sb_in_disk, sb, "Failed to initialize the super block.");
            });

        let fs = FileSystem::open(dev, true).expect("Failed to create file system.");

        // Create the root inode and initialize it.
        fs.allocate_inode(InodeType::Directory)
            .ok_or_else(|| FileSystemInitError(String::from("Failed to create the root inode.")))
    }

    /// Allocates a new empty inode from current file system.
    pub fn allocate_inode(self: &Arc<Self>, type_: InodeType) -> Option<Arc<Mutex<Inode>>> {
        if let Some(inum) = {
            let mut block_cache = self.block_cache.lock();
            block_cache
                .get(self.sb.inode_bmap_start, self.dev.clone())
                .lock()
                .write(0, |inode_bmap: &mut BitmapBlock| inode_bmap.allocate())
            // Release the lock of `block_cache` here.
        } {
            // The `inum` may be exceeding the limits of maximum number
            // of inodes, so we can't use it directly.
            if inum > self.max_inode_num() as usize {
                warn!("Failed to allocate an inode: the new `inum` exceeds the max inum of inode.");
                warn!("inum: {}, max_inum: {}", inum, self.max_inode_num());
                return None;
            }

            match self.inode_cache.lock().get(inum as InodeId, self.clone()) {
                Ok(inode) => {
                    inode
                        .lock()
                        .update_dinode(|dinode| dinode.initialize(type_));
                    Some(inode)
                }
                _ => panic!("Failed to access the inode just allocated: {}", inum),
            }
        } else {
            warn!("Failed to allocate an inode: exceeding the range of inode bit map.");
            None
        }
    }

    /// Allocates a free space in data area.
    pub fn allocate_block(self: &Arc<Self>) -> Option<BlockId> {
        for bmap_block_id in self.sb.data_bmap_start..self.sb.data_start {
            let bmap_offset = bmap_block_id - self.sb.data_bmap_start;
            if let Some(offset) = self
                .block_cache
                .lock()
                .get(bmap_block_id, self.dev.clone())
                .lock()
                .write(0, |data_bmap: &mut BitmapBlock| data_bmap.allocate())
            {
                let block_id =
                    self.sb.data_start + bmap_offset * BLOCK_SIZE as u64 + offset as BlockId;
                if block_id >= self.sb.data_start + self.sb.data_blocks {
                    warn!("fs: block id exceeds the range of data blocks.");
                    return None;
                }
                return Some(block_id);
            }
        }
        warn!("fs: can't find an available block.");
        None
    }

    /// Gets the root inode.
    ///
    /// # Safety
    /// Panics when the root inode has not been created.
    pub fn root(self: &Arc<Self>) -> Arc<Mutex<Inode>> {
        self.get_inode(0).unwrap()
    }

    fn get_inode(self: &Arc<Self>, inum: InodeId) -> Result<Arc<Mutex<Inode>>, InodeNotExists> {
        self.inode_cache.lock().get(inum, self.clone())
    }

    fn max_inode_num(self: &Arc<Self>) -> InodeId {
        self.sb.inode_blocks * (INODES_PER_BLOCK as u64)
    }
}

#[derive(Debug)]
pub struct FileSystemInitError(String);

#[derive(Debug)]
pub struct FileSystemInvalid();

#[derive(Debug)]
pub enum FileSystemAllocationError {
    Exhausted(usize),
    InodeExhausted,
    AlreadyExist(String, InodeType),
    TooLarge(usize),
}
