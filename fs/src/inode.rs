use core::slice::{from_raw_parts, from_raw_parts_mut};

use alloc::{string::ToString, sync::Arc};

use crate::{
    block_dev::{
        BlockDevice, BlockId, DInode, DirEntry, InBlockOffset, InodeId, InodeType, BLOCK_SIZE,
        DIR_ENTRY_SIZE, MAX_SIZE_ONE_INODE,
    },
    buffer_cache::block_cache,
    FileSystem, FileSystemAllocationError,
};

/// In-memory copy of an inode.
///
/// Inode (i.e. Index Node) is a structure provides information
/// for each file or directory. It includes all metadata we could
/// see by `stat` command, like size, permission, type and
/// the index of data block.
pub struct Inode {
    /// Block device.
    dev:             Arc<dyn BlockDevice>,
    /// File system.
    fs:              Arc<FileSystem>,
    /// Block id.
    block_id:        BlockId,
    /// Block offset.
    in_block_offset: InBlockOffset,
    /// Inode number.
    pub inode_num:   InodeId,
}

impl Inode {
    pub fn from_inum(
        inode_num: InodeId,
        dev: Arc<dyn BlockDevice>,
        fs: Arc<FileSystem>,
    ) -> Arc<Self> {
        let (block_id, in_block_offset) = fs.inode_pos(inode_num);
        Arc::new(Self {
            dev,
            fs,
            block_id,
            in_block_offset,
            inode_num,
        })
    }

    pub fn from_path(path: &str, start_at: Arc<Inode>) -> Option<Arc<Self>> {
        let mut ip = start_at;
        let mut path = path;

        while let Some((name, next_path)) = skip(path) {
            if ip.type_() != InodeType::Directory {
                return None;
            }

            if let Some(next) = ip.look_up(name) {
                ip = next;
            } else {
                return None;
            }

            path = next_path;
        }

        Some(ip)
    }

    pub fn read_dinode<V>(&self, f: impl FnOnce(&DInode) -> V) -> V {
        block_cache(self.block_id, self.dev.clone())
            .lock()
            .read(self.in_block_offset, f)
    }

    pub fn write_dinode<V>(&self, f: impl FnOnce(&mut DInode) -> V) -> V {
        block_cache(self.block_id, self.dev.clone())
            .lock()
            .write(self.in_block_offset, f)
    }

    pub fn is_valid(&self) -> bool {
        self.read_dinode(|dinode| dinode.is_valid())
    }

    pub fn type_(&self) -> InodeType {
        self.read_dinode(|dinode| dinode.type_)
    }

    pub fn size(&self) -> usize {
        self.read_dinode(|dinode| dinode.size as usize)
    }

    pub fn links_num(&self) -> usize {
        self.read_dinode(|dinode| dinode.links_num as usize)
    }

    pub fn look_up(&self, name: &str) -> Option<Arc<Inode>> {
        assert_eq!(self.type_(), InodeType::Directory);

        let files_num = self.size() / DIR_ENTRY_SIZE;

        let dirent = &mut DirEntry::empty();
        for i in 0..files_num {
            let read_size = self.read_data(DIR_ENTRY_SIZE * i, unsafe {
                from_raw_parts_mut(dirent as *mut _ as *mut u8, DIR_ENTRY_SIZE)
            });

            assert_eq!(read_size, DIR_ENTRY_SIZE);

            if dirent.name() == name {
                return Some(Inode::from_inum(dirent.inode_num, self.dev.clone(), self.fs.clone()));
            }
        }

        None
    }

    /// Allocates a new empty inode under this inode directory.
    pub fn allocate(
        &self,
        name: &str,
        type_: InodeType,
    ) -> Result<Arc<Inode>, FileSystemAllocationError> {
        assert_eq!(self.type_(), InodeType::Directory);

        match self.look_up(name) {
            Some(quality) => {
                if quality.type_() == type_ {
                    return Err(FileSystemAllocationError::AlreadyExist(name.to_string(), type_));
                }
            }
            _ => {}
        }

        let inode = self
            .fs
            .allocate_inode(type_)
            .ok_or_else(|| FileSystemAllocationError::InodeExhausted)?;

        let offset = self.size();
        self.resize(offset + DIR_ENTRY_SIZE)?;
        assert_eq!(self.size(), offset + DIR_ENTRY_SIZE);

        let written = self.write_data(offset, unsafe {
            let dirent = &DirEntry::new(name, inode.inode_num);
            from_raw_parts(dirent as *const _ as *const u8, DIR_ENTRY_SIZE)
        });
        assert_eq!(written, DIR_ENTRY_SIZE);

        Ok(inode)
    }

    /// Reads data from this inode to buffer.
    ///
    /// Returns the size of read data.
    pub fn read_data(&self, offset: usize, buf: &mut [u8]) -> usize {
        self.read_dinode(|dinode| dinode.read_data(offset, buf, self.dev.clone()))
    }

    /// Writes data from buffer to inode.
    ///
    /// Returns the size of written data.
    pub fn write_data(&self, offset: usize, buf: &[u8]) -> usize {
        self.write_dinode(|dinode| dinode.write_data(offset, buf, self.dev.clone()))
    }

    fn set_size(&self, size: usize) {
        self.write_dinode(|dinode| {
            dinode.size = size as u32;
        });
    }

    pub fn resize(&self, new_size: usize) -> Result<(), FileSystemAllocationError> {
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

            for i in 0..needed_blocks {
                let block_id = self
                    .fs
                    .allocate()
                    .ok_or_else(|| FileSystemAllocationError::Exhausted(new_size))?;

                self.write_dinode(|dinode| {
                    dinode.set_block_id(base_idx + i, block_id, self.dev.clone());
                })
            }

            self.set_size(new_size);
            Ok(())
        } else if new_size < old_size {
            unimplemented!()
        } else {
            Ok(()) // invariable size
        }
    }
}

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
