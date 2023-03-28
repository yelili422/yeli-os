use core::slice::{from_raw_parts, from_raw_parts_mut};

use alloc::{
    string::ToString,
    sync::{Arc, Weak},
    vec::Vec,
};
use log::warn;
use spin::Mutex;

use crate::{
    block_dev::{
        BlockId, DInode, DirEntry, InBlockOffset, InodeId, InodeType, BLOCK_SIZE, DIR_ENTRY_SIZE,
        INODES_PER_BLOCK, MAX_SIZE_ONE_INODE, N_DIRECT,
    },
    FileSystem, FileSystemAllocationError,
};

const INODE_BUFFER_SIZE: usize = 64;

/// Inodes cache.
///
/// Keeps a cache of in-use inodes in memory to provide a place
/// for synchronizing access to inodes used by multiple processes.
pub struct InodeCacheBuffer {
    cache: Vec<(InodeId, Arc<Mutex<Inode>>)>,
}

impl InodeCacheBuffer {
    pub fn new() -> Self {
        Self { cache: Vec::new() }
    }

    pub fn get(
        &mut self,
        inum: InodeId,
        fs: Arc<FileSystem>,
    ) -> Result<Arc<Mutex<Inode>>, InodeNotExists> {
        if inum > fs.sb.inode_blocks_num * INODES_PER_BLOCK as u32 {
            warn!(
                "try to obtain an inode out of the range, inum: {}, max_inode_num: {}",
                inum, fs.sb.inode_blocks_num
            );
            return Err(InodeNotExists(inum));
        }

        if self.cache.len() == INODE_BUFFER_SIZE {
            // TODO:
            unimplemented!();
        }

        if let Some((_, inode)) = self.cache.iter().find(|(id, _)| *id == inum) {
            Ok(inode.clone())
        } else {
            let (block_id, in_block_offset) = fs.sb.inode_pos(inum);

            // Acquire cache buffer block.
            let mut block_cache = fs.block_cache.lock();

            // Acquire block cache lock.
            let block_lock = block_cache.get(block_id, fs.dev.clone());
            let block = block_lock.lock();

            let dinode = unsafe { block.get_ref::<DInode>(in_block_offset) };
            let inode = Arc::new(Mutex::new(Inode::new(
                Arc::downgrade(&fs),
                block_id,
                in_block_offset,
                inum,
                dinode,
            )));

            self.cache.push((inum, inode.clone()));
            Ok(inode)
        }
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
    fs:              Weak<FileSystem>,
    /// Block id.
    block_id:        BlockId,
    /// Block offset.
    in_block_offset: InBlockOffset,
    /// Inode number.
    pub inode_num:   InodeId,

    // Copy of `DInode`.
    /// File type.
    type_:     InodeType,
    /// Major device number.
    major:     InodeId,
    /// Minor device number.
    minor:     InodeId,
    /// Counts the number of directory entries that refer to this inode.
    links_num: u32,
    /// Size of file (bytes).
    size:      u32,
    /// Data block addresses.
    addresses: [BlockId; N_DIRECT],
}

impl Inode {
    // pub fn from_path(path: &str, start_at: Arc<Mutex<Inode>>) -> Option<Arc<Mutex<Self>>> {
    //     let mut ip_lock = &start_at;
    //     let mut path = path;

    //     while let Some((name, next_path)) = skip(path) {
    //         let ip = ip_lock.lock();

    //         if ip.type_() != InodeType::Directory {
    //             return None;
    //         }

    //         if let Some(next) = ip.look_up(name) {
    //             ip_lock = &next;
    //         } else {
    //             return None;
    //         }

    //         path = next_path;
    //     }

    //     Some(ip_lock.clone())
    // }

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
            major: dinode.major,
            minor: dinode.minor,
            links_num: dinode.links_num,
            size: dinode.size,
            addresses: dinode.addresses,
        }
    }

    pub fn size(&self) -> usize {
        self.size as usize
    }

    fn dinode(&self) -> DInode {
        DInode::new(self.type_, self.major, self.minor, self.links_num, self.size, self.addresses)
    }

    pub fn update_dinode<V>(&mut self, f: impl FnOnce(&mut DInode) -> V) -> V {
        let fs = self.fs.upgrade().unwrap();

        let cache_lock = fs.block_cache.lock().get(self.block_id, fs.dev.clone());
        let mut dinode_cache = cache_lock.lock();

        let execute_then_update = |dinode: &mut DInode| {
            let callback_ret = f(dinode);

            // Update the fields in `Inode`.
            self.type_ = dinode.type_;
            self.major = dinode.major;
            self.minor = dinode.minor;
            self.links_num = dinode.links_num;
            self.size = dinode.size;
            self.addresses = dinode.addresses;

            callback_ret
        };
        dinode_cache.write(self.in_block_offset, execute_then_update)
    }

    fn set_size(&mut self, size: usize) {
        self.update_dinode(|dinode| {
            dinode.size = size as u32;
        });
    }

    pub fn is_valid(&self) -> bool {
        self.type_ != InodeType::Invalid
    }

    pub fn look_up(&self, name: &str) -> Option<Arc<Mutex<Inode>>> {
        assert_eq!(self.type_, InodeType::Directory);

        let files_num = self.size() / DIR_ENTRY_SIZE;
        let fs = self.fs.upgrade().unwrap();

        let dirent = &mut DirEntry::empty();
        for i in 0..files_num {
            let read_size = self.read_data(DIR_ENTRY_SIZE * i, unsafe {
                from_raw_parts_mut(dirent as *mut _ as *mut u8, DIR_ENTRY_SIZE)
            });

            assert_eq!(read_size, DIR_ENTRY_SIZE);

            if dirent.name() == name {
                let inode = fs
                    .get_inode(dirent.inode_num)
                    .expect("failed to get an inode from the directory entry.");
                return Some(inode);
            }
        }

        None
    }

    /// Creates a new empty inode under this inode directory.
    pub fn create(
        &mut self,
        name: &str,
        type_: InodeType,
    ) -> Result<Arc<Mutex<Inode>>, FileSystemAllocationError> {
        // FIXME:
        assert_eq!(self.type_, InodeType::Directory);

        match self.look_up(name) {
            Some(quality_lock) => {
                let quality = quality_lock.lock();
                if quality.type_ == type_ {
                    return Err(FileSystemAllocationError::AlreadyExist(name.to_string(), type_));
                }
            }
            _ => {}
        }

        let fs = self.fs.upgrade().unwrap();
        let inode_lock = fs
            .allocate_inode(type_)
            .ok_or_else(|| FileSystemAllocationError::InodeExhausted)?;

        let offset = self.size();
        self.resize(offset + DIR_ENTRY_SIZE)?;
        assert_eq!(self.size(), offset + DIR_ENTRY_SIZE);

        {
            let mut inode = inode_lock.lock();

            let written = self.write_data(offset, unsafe {
                let dirent = &DirEntry::new(name, inode.inode_num);
                from_raw_parts(dirent as *const _ as *const u8, DIR_ENTRY_SIZE)
            });
            assert_eq!(written, DIR_ENTRY_SIZE);

            inode.update_dinode(|dinode| dinode.links_num += 1);
        }

        Ok(inode_lock)
    }

    /// Reads data from this inode to buffer.
    ///
    /// Returns the size of read data.
    pub fn read_data(&self, offset: usize, buf: &mut [u8]) -> usize {
        let fs = self.fs.upgrade().unwrap();
        self.dinode()
            .read_data(offset, buf, fs.dev.clone(), fs.block_cache.clone())
    }

    /// Writes data from buffer to inode.
    ///
    /// Returns the size of written data.
    pub fn write_data(&self, offset: usize, buf: &[u8]) -> usize {
        let fs = self.fs.upgrade().unwrap();
        self.dinode()
            .write_data(offset, buf, fs.dev.clone(), fs.block_cache.clone())
    }

    pub fn resize(&mut self, new_size: usize) -> Result<(), FileSystemAllocationError> {
        if new_size > MAX_SIZE_ONE_INODE {
            return Err(FileSystemAllocationError::TooLarge(new_size));
        }

        let old_size = self.size();
        if new_size > old_size {
            let in_block_offset = old_size % BLOCK_SIZE;
            let mut increment = new_size - old_size;

            if in_block_offset != 0 {
                // has remaining space
                if increment > BLOCK_SIZE - in_block_offset {
                    increment -= BLOCK_SIZE - in_block_offset;
                } else {
                    self.set_size(new_size);
                    return Ok(());
                }
            }

            let base_idx = (old_size + BLOCK_SIZE - 1) / BLOCK_SIZE;
            let needed_blocks = (increment + BLOCK_SIZE - 1) / BLOCK_SIZE;

            let fs = self.fs.upgrade().unwrap();
            for i in 0..needed_blocks {
                let block_id = fs
                    .allocate_block()
                    .ok_or_else(|| FileSystemAllocationError::Exhausted(new_size))?;

                clear_block(block_id, fs.clone());

                self.update_dinode(|dinode| {
                    dinode.set_block_id(
                        base_idx + i,
                        block_id,
                        fs.dev.clone(),
                        fs.block_cache.clone(),
                    );
                })
            }

            self.set_size(new_size);
            Ok(())
        } else if new_size < old_size {
            unimplemented!()
        } else {
            Ok(()) // invariant size
        }
    }
}

/// The inode doesn't exists.
#[derive(Debug, Clone, Copy)]
pub struct InodeNotExists(InodeId);

/// Skips the next path element.
///
/// Returns next path element and the element following that.
/// If no next path element, return `None`.
///
/// # Examples
///
/// ```no_run
/// assert_eq!(skip("a/bb/c"), Some(("a", "bb/c")));
/// assert_eq!(skip("///a/bb"), Some(("a", "bb")));
/// assert_eq!(skip("a"), Some(("a", "")));
/// assert_eq!(skip(""), None);
/// ```
///
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

fn clear_block(bid: BlockId, fs: Arc<FileSystem>) {
    let block_lock = fs.block_cache.lock().get(bid, fs.dev.clone());
    {
        let mut block = block_lock.lock();
        block.clear();
        block.sync();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip() {
        assert_eq!(skip("a/bb/c"), Some(("a", "bb/c")));
        assert_eq!(skip("///a/bb"), Some(("a", "bb")));
        assert_eq!(skip("a"), Some(("a", "")));
        assert_eq!(skip(""), None);
    }
}
