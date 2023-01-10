use core::{mem::size_of, slice::from_raw_parts_mut};

use alloc::{string::String, sync::Arc};

use crate::{
    block_dev::{
        BlockDevice, BlockId, DInode, DirEntry, InBlockOffset, InodeId, InodeType, SuperBlock,
    },
    buffer_cache::block_cache,
};

/// In-memory copy of an inode.
///
/// Inode (i.e. Index Node) is a structure provides information
/// for each file or directory. It includes all metadata we could
/// see by `stat` command, like size, permission, type and
/// the index of data block.
pub struct Inode {
    /// Block device.
    dev:          Arc<dyn BlockDevice>,
    /// Super block.
    super_block:  Arc<SuperBlock>,
    /// Block id.
    block_id:     BlockId,
    /// Block offset.
    block_offset: InBlockOffset,
    /// Inode number.
    inode_num:    InodeId,
    // TODO: a fs global lock.
}

impl Inode {
    pub fn from_inum(
        inode_num: InodeId,
        dev: Arc<dyn BlockDevice>,
        super_block: Arc<SuperBlock>,
    ) -> Self {
        let (block_id, block_offset) = super_block.inode_pos(inode_num);
        Self {
            dev,
            block_id,
            block_offset,
            super_block,
            inode_num,
        }
    }

    pub fn from_path(path: &str, start_at: Inode) -> Option<Self> {
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

    fn read_dinode<V>(&self, f: impl FnOnce(&DInode) -> V) -> V {
        block_cache(self.block_id, Arc::clone(&self.dev))
            .lock()
            .read_from(self.block_offset, f)
    }

    fn write_dinode<V>(&self, f: impl FnOnce(&mut DInode) -> V) -> V {
        block_cache(self.block_id, Arc::clone(&self.dev))
            .lock()
            .write_to(self.block_offset, f)
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

    pub fn look_up(&self, name: &str) -> Option<Inode> {
        assert_eq!(self.type_(), InodeType::Directory);

        let dirent_size = size_of::<DirEntry>();
        let files_num = self.size() / dirent_size;

        let dirent = &mut DirEntry::empty();
        for i in 0..files_num {
            let read_size = self.read(dirent_size * i, unsafe {
                from_raw_parts_mut(dirent as *mut _ as *mut u8, dirent_size)
            });

            assert_eq!(read_size, dirent_size);

            if String::from(dirent.name()) == name {
                return Some(Inode::from_inum(
                    dirent.inode_num,
                    Arc::clone(&self.dev),
                    Arc::clone(&self.super_block),
                ));
            }
        }

        None
    }

    /// Reads data from this inode to buffer.
    ///
    /// Returns the size of read data.
    pub fn read(&self, offset: usize, buf: &mut [u8]) -> usize {
        self.read_dinode(|dinode| dinode.read(offset, buf, &self.dev))
    }

    /// Writes data from buffer to inode.
    ///
    /// Returns the size of written data.
    pub fn write(&self, offset: usize, buf: &[u8]) -> usize {
        self.write_dinode(|dinode| dinode.write(offset, buf, &self.dev))
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
