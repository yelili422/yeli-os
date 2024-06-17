use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};
use log::{debug, warn};
use spin::Mutex;

use crate::{
    block_dev::{BlockId, DInode, InBlockOffset, InodeId, InodeType, N_DIRECT},
    FileSystem,
};

pub const INODE_BUFFER_SIZE: usize = 64;

/// Inodes cache.
///
/// Keeps a cache of in-use inodes in memory to provide a place
/// for synchronizing access to inodes used by multiple processes.
pub struct InodeCacheBuffer {
    cache:    Vec<(InodeId, Arc<Mutex<Inode>>)>,
    capacity: usize,
}

impl InodeCacheBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Vec::new(),
            capacity,
        }
    }

    pub fn get(
        &mut self,
        inum: InodeId,
        fs: Arc<FileSystem>,
    ) -> Result<Arc<Mutex<Inode>>, InodeNotExists> {
        if inum > fs.max_inode_num() {
            warn!(
                "try to obtain an inode out of the range, inum: {}, max_inode_num: {}",
                inum,
                fs.max_inode_num()
            );
            return Err(InodeNotExists(inum));
        }

        if self.cache.len() == self.capacity {
            let (id, _) = self.cache.remove(self.capacity - 1);
            debug!("remove inode {} from cache", id);
        }

        let inode = match self.cache.iter().position(|&(id, _)| id == inum) {
            Some(pos) => {
                let (_, inode) = self.cache.remove(pos);
                inode
            }
            None => {
                let (block_id, in_block_offset) = fs.sb.find_inode(inum);

                // Acquire cache buffer block.
                let mut block_cache = fs.block_cache.lock();

                // Acquire block cache lock.
                let block_lock = block_cache.get(block_id, fs.dev.clone());
                let block = block_lock.lock();

                let dinode = unsafe { block.get_ref::<DInode>(in_block_offset) };
                Arc::new(Mutex::new(Inode::new(
                    Arc::downgrade(&fs),
                    block_id,
                    in_block_offset,
                    inum,
                    dinode,
                )))
            }
        };
        self.cache.insert(0, (inum, inode.clone()));
        Ok(inode)
    }
}

/// In-memory copy of an inode.
///
/// Inode (i.e. Index Node) is a structure provides information
/// for each file or directory. It describes a single unnamed file
/// and holds metadata we could see by `stat` command, like size,
/// permission, type and the index of data block.
pub struct Inode {
    /// File system
    fs:                  Weak<FileSystem>,
    /// Block id.
    pub block_id:        BlockId,
    /// Block offset.
    pub in_block_offset: InBlockOffset,
    /// Inode number.
    pub inode_num:       InodeId,

    // Copy of `DInode`.
    /// File type.
    pub type_: InodeType,
    /// Indirect block number.
    indirect:  InodeId,
    /// Counts the number of directory entries that refer to this inode.
    links_num: u64,
    /// Size of file (bytes).
    size:      u64,
    /// Data block addresses.
    addresses: [BlockId; N_DIRECT],
}

impl Inode {
    fn new(
        fs: Weak<FileSystem>,
        block_id: BlockId,
        in_block_offset: InBlockOffset,
        inode_num: InodeId,
        dinode: &DInode,
    ) -> Self {
        Self {
            fs,
            block_id,
            in_block_offset,
            inode_num,
            type_: dinode.type_,
            indirect: dinode.indirect,
            links_num: dinode.links_num,
            size: dinode.size,
            addresses: dinode.addresses,
        }
    }

    pub fn get_fs(&self) -> Option<Arc<FileSystem>> {
        self.fs.upgrade()
    }

    pub fn size(&self) -> usize {
        self.size as usize
    }

    pub fn dinode(&self) -> DInode {
        DInode::new(self.type_, self.indirect, self.links_num, self.size, self.addresses)
    }

    pub fn is_valid(&self) -> bool {
        self.type_ != InodeType::Invalid
    }

    pub fn update(&mut self, dinode: &DInode) {
        self.type_ = dinode.type_;
        self.indirect = dinode.indirect;
        self.links_num = dinode.links_num;
        self.size = dinode.size;
        self.addresses = dinode.addresses;
    }
}

/// The inode doesn't exists.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct InodeNotExists(InodeId);
