#![no_std]

extern crate alloc;

use alloc::{
    string::{String, ToString},
    sync::Arc,
};
use block_cache::{BlockCacheBuffer, BLOCK_BUFFER_SIZE};
use block_dev::{
    BitmapBlock, BlockDevice, BlockId, DInode, DirEntry, InodeId, InodeType, SuperBlock,
    BLOCK_SIZE, CAPACITY_PER_INODE, DINODE_SIZE, DIR_ENTRY_SIZE, INODES_PER_BLOCK,
    MAX_BLOCKS_PER_INODE,
};
use core::{
    cmp::min,
    mem::size_of,
    slice::{from_raw_parts, from_raw_parts_mut},
};
use inode::{Inode, InodeCacheBuffer, InodeNotExists, INODE_BUFFER_SIZE};
use log::{debug, warn};
use spin::{Mutex, MutexGuard};

pub mod block_cache;
pub mod block_dev;
pub mod inode;

#[cfg(test)]
mod helpers;

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

    /// Create file system on given block device.
    pub fn create(
        dev: Arc<dyn BlockDevice>,
        total_blocks: u64,
        inode_blocks: u64,
    ) -> Result<Arc<Self>, FileSystemInitError> {
        let mut rest_blocks = total_blocks;

        debug!("fs: block_size: {} Bytes", BLOCK_SIZE);
        debug!("fs: inode_size: {} Bytes", DINODE_SIZE);
        assert_eq!(
            DINODE_SIZE,
            BLOCK_SIZE / INODES_PER_BLOCK,
            "The size of the inode needs to be adapted to the `block_size`"
        );

        debug!("fs: max data blocks of one inode: {}", MAX_BLOCKS_PER_INODE);
        debug!(
            "fs: max data size of one inode: {} Bytes({} MBytes)",
            CAPACITY_PER_INODE,
            CAPACITY_PER_INODE / 1024 / 1024
        );

        let super_blocks = 1;
        let logging_blocks = 1;
        debug!("fs: super_block: {}", super_blocks);
        debug!("fs: logging_blocks: {}", logging_blocks);
        rest_blocks -= super_blocks + logging_blocks;

        let inode_bmap_blocks = inode_blocks / (size_of::<BitmapBlock>() as u64) + 1;
        let inode_area = inode_bmap_blocks + inode_blocks;
        debug!("fs: total blocks: {}", total_blocks);
        debug!(
            "fs: inode area: inode_bitmap_blocks({}) + inode_blocks({})",
            inode_bmap_blocks, inode_blocks
        );

        assert!(rest_blocks > inode_area, "No more space for data blocks.");
        rest_blocks -= inode_area;

        let data_bmap_blocks = rest_blocks / (BLOCK_SIZE as u64) / 8 + 1;
        let data_blocks_num = rest_blocks - data_bmap_blocks;

        debug!(
            "fs: data area: data bitmap({}) + data blocks({})",
            data_bmap_blocks, data_blocks_num
        );

        let inode_bmap_start = SUPER_BLOCK_LOC + super_blocks;
        let inode_start = inode_bmap_start + inode_bmap_blocks;
        let data_bmap_start = inode_start + inode_blocks;
        let data_start = data_bmap_start + data_bmap_blocks;

        let sb = SuperBlock::new(
            total_blocks,
            inode_bmap_start,
            inode_start,
            inode_blocks,
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
        let block_cache = Arc::new(Mutex::new(BlockCacheBuffer::new(BLOCK_BUFFER_SIZE)));
        let inode_cache = Arc::new(Mutex::new(InodeCacheBuffer::new(INODE_BUFFER_SIZE)));

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
        let block_cache = Arc::new(Mutex::new(BlockCacheBuffer::new(BLOCK_BUFFER_SIZE)));

        // Clear all non-data blocks.
        for i in sb.inode_bmap_start..sb.data_start {
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
        match self.allocate_bmap(self.sb.inode_bmap_start, self.sb.inode_start) {
            Some(inum) => {
                if inum >= self.max_inode_num() {
                    warn!(
                        "fs: allocate_id exceeds the range of inodes. {}, max_inode_num: {}",
                        inum,
                        self.max_inode_num()
                    );
                    None
                } else {
                    match self.inode_cache.lock().get(inum, self.clone()) {
                        Ok(inode_lock) => {
                            let inode_lock_clone = inode_lock.clone();
                            let mut inode_clone = inode_lock_clone.lock();
                            self.update_dinode(&mut inode_clone, |dinode| dinode.initialize(type_));
                            Some(inode_lock)
                        }
                        _ => panic!("Failed to access the inode just allocated: {}", inum),
                    }
                }
            }
            None => {
                warn!("fs: can't allocate blocks because of inode bitmap exhausted.");
                None
            }
        }
    }

    /// Allocates a free space in data area.
    pub fn allocate_data_block(self: &Arc<Self>) -> Option<BlockId> {
        match self.allocate_bmap(self.sb.data_bmap_start, self.sb.data_start) {
            Some(allocate_id) => {
                if allocate_id >= self.sb.data_blocks {
                    warn!("fs: allocate_id exceeds the range of data blocks. {}", allocate_id);
                    None
                } else {
                    Some(self.sb.data_start + allocate_id)
                }
            }
            None => {
                warn!("fs: can't allocate blocks because of data bitmap exhausted.");
                None
            }
        }
    }

    fn allocate_bmap(self: &Arc<Self>, start: BlockId, end: BlockId) -> Option<u64> {
        for i in start..end {
            let block_offset = i - start;
            let offset = self
                .block_cache
                .lock()
                .get(i, self.dev.clone())
                .lock()
                .write(0, |bmap: &mut BitmapBlock| bmap.allocate());
            if let Some(offset) = offset {
                return Some(block_offset * 8 * BLOCK_SIZE as u64 + offset as u64);
            }
        }
        None
    }

    pub fn max_blocks_num(self: &Arc<Self>) -> u64 {
        min(self.sb.data_blocks, self.sb.inode_blocks * MAX_BLOCKS_PER_INODE as u64)
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

    fn update_dinode<V>(
        self: &Arc<Self>,
        inode: &mut MutexGuard<Inode>,
        f: impl FnOnce(&mut DInode) -> V,
    ) -> V {
        let cache_lock = self
            .block_cache
            .lock()
            .get(inode.block_id, self.dev.clone());
        let mut dinode_cache = cache_lock.lock();

        let offset = inode.in_block_offset;
        let execute_then_update = |dinode: &mut DInode| {
            let callback_ret = f(dinode);
            inode.update(dinode);

            callback_ret
        };
        dinode_cache.write(offset, execute_then_update)
    }

    fn set_inode_size(self: &Arc<Self>, inode: &mut MutexGuard<Inode>, size: usize) {
        self.update_dinode(inode, |dinode| {
            dinode.size = size as u64;
        });
    }

    pub fn look_up(
        self: &Arc<Self>,
        inode: &MutexGuard<Inode>,
        name: &str,
    ) -> Option<Arc<Mutex<Inode>>> {
        assert_eq!(inode.type_, InodeType::Directory, "Only directories can look up files.");

        let files_num = inode.size() / DIR_ENTRY_SIZE;
        let dirent = &mut DirEntry::empty();

        // TODO: Looking up a file by name will be slow when files_num
        // more and more bigger.
        for i in 0..files_num {
            let read_size = self.read_inode(&inode, DIR_ENTRY_SIZE * i, unsafe {
                from_raw_parts_mut(dirent as *mut _ as *mut u8, DIR_ENTRY_SIZE)
            });

            assert_eq!(read_size, DIR_ENTRY_SIZE);

            if dirent.name() == name {
                let inode = self
                    .get_inode(dirent.inode_num)
                    .expect("failed to get an inode from the directory entry.");
                return Some(inode);
            }
        }

        None
    }

    /// Creates a new empty inode under this inode directory.
    pub fn create_inode(
        self: &Arc<Self>,
        inode: &mut MutexGuard<Inode>,
        name: &str,
        type_: InodeType,
    ) -> Result<Arc<Mutex<Inode>>, FileSystemAllocationError> {
        assert_eq!(
            inode.type_,
            InodeType::Directory,
            "New files only can be created in directories."
        );

        if let Some(_) = self.look_up(inode, name) {
            return Err(FileSystemAllocationError::AlreadyExist(name.to_string(), type_));
        }

        let new_inode_lock = self
            .allocate_inode(type_)
            .ok_or_else(|| FileSystemAllocationError::InodeExhausted)?;

        let base_offset = inode.size();
        self.resize_inode(inode, base_offset + DIR_ENTRY_SIZE)?;
        assert_eq!(inode.size(), base_offset + DIR_ENTRY_SIZE);

        let mut new_inode = new_inode_lock.lock();
        {
            let dirent = &DirEntry::new(name, new_inode.inode_num);

            let written = self.write_inode(inode, base_offset, unsafe {
                from_raw_parts(dirent as *const _ as *const u8, DIR_ENTRY_SIZE)
            });
            assert_eq!(written, DIR_ENTRY_SIZE);

            self.update_dinode(&mut new_inode, |dinode| dinode.links_num += 1);
        }

        Ok(new_inode_lock.clone())
    }

    /// Reads data from this inode to buffer.
    ///
    /// Returns the size of read data.
    pub fn read_inode(&self, inode: &MutexGuard<Inode>, offset: usize, buf: &mut [u8]) -> usize {
        inode
            .dinode()
            .read_data(offset, buf, self.dev.clone(), self.block_cache.clone())
    }

    /// Writes data from buffer to inode.
    ///
    /// Returns the size of written data.
    pub fn write_inode(&self, inode: &MutexGuard<Inode>, offset: usize, buf: &[u8]) -> usize {
        inode
            .dinode()
            .write_data(offset, buf, self.dev.clone(), self.block_cache.clone())
    }

    pub fn resize_inode(
        self: &Arc<Self>,
        inode: &mut MutexGuard<Inode>,
        new_size: usize,
    ) -> Result<(), FileSystemAllocationError> {
        if new_size > CAPACITY_PER_INODE {
            return Err(FileSystemAllocationError::TooLarge(new_size));
        }

        let old_size = inode.size();
        debug!(
            "inode: resize inode {} from {} Bytes to {} Bytes ({:.6} MBytes)",
            inode.inode_num,
            old_size,
            new_size,
            (new_size as f64) / 1024. / 1024.
        );
        if new_size > old_size {
            let in_block_offset = old_size % BLOCK_SIZE;
            let mut increment = new_size - old_size;

            if in_block_offset != 0 {
                // has remaining space
                if increment > BLOCK_SIZE - in_block_offset {
                    increment -= BLOCK_SIZE - in_block_offset;
                } else {
                    self.set_inode_size(inode, new_size);
                    return Ok(());
                }
            }

            let base_idx = (old_size + BLOCK_SIZE - 1) / BLOCK_SIZE;
            let needed_blocks = (increment + BLOCK_SIZE - 1) / BLOCK_SIZE;
            debug!("inode: allocate new blocks, needs {}", needed_blocks);

            for i in 0..needed_blocks {
                let block_id = self
                    .allocate_data_block()
                    .ok_or_else(|| FileSystemAllocationError::Exhausted(new_size))?;
                debug!("inode: resize: allocated block_id: {}", block_id);
                clear_block(block_id, self.clone());

                self.update_dinode(inode, |dinode| {
                    dinode.set_bid(
                        base_idx + i,
                        block_id,
                        self.dev.clone(),
                        self.block_cache.clone(),
                    );
                })
            }

            self.set_inode_size(inode, new_size);
            Ok(())
        } else if new_size < old_size {
            unimplemented!()
        } else {
            Ok(()) // invariant size
        }
    }

    pub fn get_inode_from_path(
        self: &Arc<Self>,
        path: &str,
        start_at: &Arc<Mutex<Inode>>,
    ) -> Option<Arc<Mutex<Inode>>> {
        if path.is_empty() {
            return Some(start_at.clone());
        }

        while let Some((name, next_path)) = skip(path) {
            let ip = start_at.lock();
            if ip.type_ != InodeType::Directory {
                return None;
            }

            if let Some(next_ip) = self.look_up(&ip, name) {
                return self.get_inode_from_path(next_path, &next_ip);
            } else {
                return None;
            }
        }

        None
    }
}

#[allow(dead_code)]
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

fn clear_block(bid: BlockId, fs: Arc<FileSystem>) {
    let block_lock = fs.block_cache.lock().get(bid, fs.dev.clone());
    {
        let mut block = block_lock.lock();
        block.clear();
        block.sync();
    }
}

// Skips the next path element.
//
// Returns next path element and the element following that.
// If no next path element, return `None`.
//
// # Examples
//
// ```
// assert_eq!(skip("a/bb/c"), Some(("a", "bb/c")));
// assert_eq!(skip("///a/bb"), Some(("a", "bb")));
// assert_eq!(skip("a"), Some(("a", "")));
// assert_eq!(skip(""), None);
// ```
//
#[allow(dead_code)]
fn skip(path: &str) -> Option<(&str, &str)> {
    let mut p = 0;

    while p < path.len() && &path[p..p + 1] == "/" {
        p += 1;
    }

    if p == path.len() {
        return None;
    }

    let name_start = p;
    while p < path.len() && &path[p..p + 1] != "/" {
        p += 1;
    }
    let len = p - name_start;

    while p < path.len() && &path[p..p + 1] == "/" {
        p += 1;
    }

    Some((&path[name_start..name_start + len], &path[p..]))
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    #[test]
    fn test_skip() {
        assert_eq!(skip("a/bb/c"), Some(("a", "bb/c")));
        assert_eq!(skip("///a/bb"), Some(("a", "bb")));
        assert_eq!(skip("a"), Some(("a", "")));
        assert_eq!(skip(""), None);
    }

    #[test]
    fn test_it_works() {
        let fs = helpers::init_fs();
        let root_lock = fs.root();
        let root = root_lock.lock();

        assert_eq!(root.inode_num, 0);
        assert_eq!(root.size(), 0);
        assert_eq!(root.type_, InodeType::Directory);
    }

    #[test]
    fn test_allocate_block() {
        let fs = helpers::init_fs();
        debug!("fs: max blocks num: {}", fs.max_blocks_num());
        for i in 0..fs.max_blocks_num() {
            let block_id = fs.allocate_data_block();
            assert_eq!(
                block_id,
                Some(fs.sb.data_start + i),
                "Failed to allocate the {}th block",
                i
            );
        }
        assert_eq!(fs.allocate_data_block(), None, "Exceeding the max blocks num.");
    }

    #[test]
    fn test_nested_dir() {
        let fs = helpers::init_fs();
        let root_lock = fs.root();
        let mut root = root_lock.lock();

        for i in 1..10 {
            let dir_lock = fs
                .create_inode(&mut root, &i.to_string(), InodeType::Directory)
                .unwrap();
            let mut dir = dir_lock.lock();

            for j in 1..10 {
                let inner_dir_lock = fs
                    .create_inode(&mut dir, &j.to_string(), InodeType::Directory)
                    .unwrap();
                let mut inner_dir = inner_dir_lock.lock();

                for k in 1..10 {
                    let file_lock = fs
                        .create_inode(&mut inner_dir, &k.to_string(), InodeType::File)
                        .unwrap();
                    let mut file = file_lock.lock();
                    assert_eq!(file.size(), 0);

                    fs.resize_inode(&mut file, 10).unwrap();
                    assert_eq!(file.size(), 10);

                    fs.write_inode(&file, 0, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
                    let mut buffer = [0u8; 10];
                    fs.read_inode(&file, 0, &mut buffer);
                    assert_eq!(buffer, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
                }
            }
        }
    }

    #[test]
    fn test_single_large_file() {
        let fs = helpers::init_fs();
        let root_lock = fs.root();
        let mut root = root_lock.lock();

        let file_lock = fs
            .create_inode(&mut root, "a_large_file", InodeType::File)
            .unwrap();
        let mut file = file_lock.lock();
        assert_eq!(file.size(), 0);

        fs.resize_inode(&mut file, CAPACITY_PER_INODE).unwrap();
        assert_eq!(file.size(), CAPACITY_PER_INODE);

        let res = fs.resize_inode(&mut file, CAPACITY_PER_INODE + 1);
        assert!(res.is_err());
    }

    #[test]
    #[ignore = "This test will take a very long time to run"]
    fn test_amounts_of_directories() {
        let fs = helpers::init_fs();
        let root_lock = fs.root();
        let mut root = root_lock.lock();

        let dir_lock = fs
            .create_inode(&mut root, "amounts_of_directories", InodeType::Directory)
            .unwrap();
        let mut dir = dir_lock.lock();

        for i in 0..block_dev::MAX_DIRENTS_PER_INODE {
            let d_lock = fs
                .create_inode(&mut dir, &i.to_string(), InodeType::Directory)
                .unwrap();
            let d = d_lock.lock();

            assert_eq!(d.type_, InodeType::Directory);
        }
    }

    #[test]
    fn test_read_write() {
        let args: alloc::vec::Vec<_> = std::env::args().collect();
        let file_path = &args[0];

        let mut src_file = std::fs::OpenOptions::new()
            .read(true)
            .open(file_path)
            .unwrap();

        let fs = helpers::init_fs();
        let root_lock = fs.root();
        let mut root = root_lock.lock();

        let dst_file_lock = fs
            .create_inode(&mut root, "read_and_write", InodeType::File)
            .unwrap();
        let mut dst_file = dst_file_lock.lock();

        fs.resize_inode(&mut dst_file, block_dev::CAPACITY_PER_INODE)
            .unwrap();

        let mut buffer = [0u8; BLOCK_SIZE];
        let mut read_count = 0;
        loop {
            let offset = std::io::Read::read(&mut src_file, &mut buffer).unwrap();
            if offset == 0 {
                break;
            }

            fs.write_inode(&dst_file, read_count, &buffer);
            read_count += offset;

            if read_count >= CAPACITY_PER_INODE {
                break;
            }
        }
    }
}
