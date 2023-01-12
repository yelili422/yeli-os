use core::mem::size_of;

use crate::buffer_cache::block_cache;
use alloc::sync::Arc;

/// The trait of block devices.
///
/// Blocks devices only support random read and write by block.
pub trait BlockDevice: Send + Sync {
    fn read(&self, block_id: u32, buf: &mut [u8]);
    fn write(&self, block_id: u32, buf: &[u8]);
}

/// The size of one block.
pub const BLOCK_SIZE: usize = 512;

/// File system magic number for sanity check.
const FS_MAGIC: u32 = 0x102030;

/// Inodes per block.
pub const INODES_PER_BLOCK: usize = BLOCK_SIZE / DINODE_SIZE;

/// Bitmap bits per block.
pub const BITMAP_PER_BLOCK: usize = BLOCK_SIZE * 8;

/// Direct blocks per inode.
///
/// We should keep `DInode` to take up the most of space in 1/n
/// of `BLOCK_SIZE`. (i.e. `DINODE_SIZE == BLOCK_SIZE / 4`)
const N_DIRECT: usize = 27;

/// Indirect blocks per block.
const N_INDIRECT: usize = BLOCK_SIZE / size_of::<u32>();

/// The maximum data blocks of one inode.
pub const MAX_BLOCKS_ONE_INODE: usize = N_DIRECT + N_INDIRECT + N_INDIRECT * N_INDIRECT;

/// The maximum inode size.
pub const MAX_SIZE_ONE_INODE: usize = MAX_BLOCKS_ONE_INODE * BLOCK_SIZE;

/// The size of directory name.
pub const DIR_NAME_SIZE: usize = 24;

/// The size of directory entry.
pub const DIR_ENTRY_SIZE: usize = size_of::<DirEntry>();

/// The size of DInode.
pub const DINODE_SIZE: usize = size_of::<DInode>();

/// The Inode ID.
///
/// Every inode is the same size, so it is easy, given
/// a number n, to find the nth inode on the disk. In fact, this number n,
/// called the inode number or i-number, is how inodes are identified in
/// the implementation.
pub type InodeId = u32;

/// The block ID.
pub type BlockId = u32;

/// The block offset.
pub type InBlockOffset = u32;

/// Disk layout:
///
/// [ boot block | super block | inode bit map | inode blocks
///                               | data bit map | data blocks ]
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SuperBlock {
    /// Must be `FS_MAGIC`
    magic:                u32,
    /// Size of file system image (blocks).
    pub size:             u32,
    /// Number of data blocks.
    pub blocks_num:       u32,
    /// Number of inodes.
    pub inodes_num:       u32,
    /// Block number of first free inode map block.
    pub inode_bmap_start: InodeId,
    /// Block number of first inode block.
    pub inode_start:      InodeId,
    /// Block number of first free data map block.
    pub data_bmap_start:  InodeId,
    /// Block number of first data block.
    pub data_start:       InodeId,
}

impl SuperBlock {
    pub fn initialize(
        &mut self,
        size: u32,
        blocks_num: u32,
        inodes_num: u32,
        inode_bmap_start: InodeId,
        inode_start: InodeId,
        data_bmap_start: InodeId,
        data_start: InodeId,
    ) {
        self.magic = FS_MAGIC;
        self.size = size;
        self.blocks_num = blocks_num;
        self.inodes_num = inodes_num;
        self.inode_start = inode_start;
        self.data_bmap_start = data_bmap_start;
        self.data_start = data_start;
        self.inode_bmap_start = inode_bmap_start;
    }

    pub fn is_valid(&self) -> bool {
        self.magic == FS_MAGIC
    }
}

/// The type of bitmap block, group of `BLOCK_SIZE`.
#[repr(C)]
pub struct BitmapBlock([bool; BLOCK_SIZE]);

impl BitmapBlock {
    pub fn allocate(&mut self) -> Option<usize> {
        match self.0.iter().enumerate().find(|&(_, &used)| used == false) {
            Some((idx, _)) => {
                self.0[idx] = true;
                Some(idx)
            }
            None => None,
        }
    }

    pub fn free(&mut self, idx: usize) {
        assert_eq!(self.0[idx], true);
        self.0[idx] = false;
    }
}

/// The type of indirect indices block pointed by inode.
pub type IndexBlock = [InodeId; BLOCK_SIZE / size_of::<InodeId>()];

/// The type of data block.
pub type DataBlock = [u8; BLOCK_SIZE];

/// Directory entry structure.
#[repr(C)]
pub struct DirEntry {
    pub inode_num: InodeId,
    name:          [u8; DIR_NAME_SIZE],
}

impl DirEntry {
    pub const fn empty() -> Self {
        Self {
            inode_num: 0,
            name:      [0; DIR_NAME_SIZE],
        }
    }

    pub fn new(name: &str, inum: InodeId) -> Self {
        let mut bytes = [0; DIR_NAME_SIZE];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        Self {
            inode_num: inum,
            name:      bytes,
        }
    }

    pub fn name(&self) -> &str {
        let len = (0..DIR_NAME_SIZE)
            .find(|&i| self.name[i] == 0)
            .unwrap_or(DIR_NAME_SIZE);
        core::str::from_utf8(&self.name[..len]).expect("Cast [u8] to str failed.")
    }
}

/// On-disk inode structure.
///
/// The on-disk inodes are packed into a contiguous area of disk called
/// the inode blocks.
#[repr(C)]
pub struct DInode {
    /// File type.
    pub type_:     InodeType,
    /// Major device number.
    pub major:     InodeId,
    /// Minor device number.
    pub minor:     InodeId,
    /// Number of links to inode in file system.
    pub links_num: u32,
    /// Size of file (bytes).
    pub size:      u32,
    /// Data block addresses.
    pub addresses: [BlockId; N_DIRECT],
}

impl DInode {
    pub fn initialize(&mut self, type_: InodeType) {
        *self = Self {
            type_,
            major: 0,
            minor: 0,
            links_num: 0,
            size: 0,
            addresses: [0; N_DIRECT],
        }
    }

    pub fn is_valid(&self) -> bool {
        self.type_ != InodeType::Invalid
    }

    /// Gets block id by inner index.
    pub fn block_id(&self, idx: usize, block_dev: Arc<dyn BlockDevice>) -> BlockId {
        assert!(idx < MAX_BLOCKS_ONE_INODE);

        if idx < N_DIRECT {
            self.addresses[idx]
        } else if idx < N_DIRECT + N_INDIRECT {
            block_cache(self.major, block_dev.clone())
                .lock()
                .read(0, |index_block: &IndexBlock| index_block[idx - N_DIRECT])
        } else {
            let p = idx - (N_DIRECT + N_INDIRECT);
            let major_block_id = block_cache(self.minor, block_dev.clone())
                .lock()
                .read(0, |minor_block: &IndexBlock| minor_block[p / N_INDIRECT]);
            block_cache(major_block_id, block_dev.clone())
                .lock()
                .read(0, |major_block: &IndexBlock| major_block[p % N_INDIRECT])
        }
    }

    /// Sets block id to given inner index.
    pub fn set_block_id(&mut self, idx: usize, block_id: BlockId, block_dev: Arc<dyn BlockDevice>) {
        assert!(idx < MAX_BLOCKS_ONE_INODE);

        if idx < N_DIRECT {
            self.addresses[idx] = block_id;
        } else if idx < N_DIRECT + N_INDIRECT {
            block_cache(self.major, block_dev.clone())
                .lock()
                .write(0, |index_block: &mut IndexBlock| index_block[idx - N_DIRECT] = block_id)
        } else {
            let p = idx - (N_DIRECT + N_INDIRECT);
            let major_block_id = block_cache(self.minor, block_dev.clone())
                .lock()
                .read(0, |minor_block: &IndexBlock| minor_block[p / N_INDIRECT]);
            block_cache(major_block_id, block_dev.clone())
                .lock()
                .write(0, |major_block: &mut IndexBlock| major_block[p % N_INDIRECT] = block_id)
        }
    }

    /// Reads data from current disk inode to buffer.
    ///
    /// Returns the size of read data.
    pub fn read_data(
        &self,
        offset: usize,
        buf: &mut [u8],
        block_dev: Arc<dyn BlockDevice>,
    ) -> usize {
        let mut start = offset;
        // Ensure the end address does not exceed the safe range.
        let end = start + buf.len().min(self.size as usize - offset);

        let mut start_block = start / BLOCK_SIZE;
        let mut completed = 0usize;
        while start < end {
            // Growth value is the minimum of the end address or the block boundary.
            let incr = end.min((start_block + 1) * BLOCK_SIZE) - start;
            let dst = &mut buf[completed..completed + incr];

            block_cache(self.block_id(start_block, block_dev.clone()), block_dev.clone())
                .lock()
                .read(0, |data_block: &DataBlock| {
                    // Copy data from this block.
                    let src = &data_block[start % BLOCK_SIZE..start % BLOCK_SIZE + incr];
                    dst.copy_from_slice(src);
                });

            completed += incr;
            start += incr;
            start_block += 1;
        }

        completed
    }

    /// Writes data from buffer to current disk inode.
    ///
    /// Returns the size of written data.
    pub fn write_data(&self, offset: usize, buf: &[u8], block_dev: Arc<dyn BlockDevice>) -> usize {
        let mut start = offset;
        // Ensure the end address does not exceed the safe range.
        let end = start + buf.len().min(self.size as usize - offset);

        let mut start_block = start / BLOCK_SIZE;
        let mut completed = 0usize;
        while start < end {
            // Growth value is the minimum of the end address or the block boundary.
            let incr = end.min((start_block + 1) * BLOCK_SIZE) - start;

            block_cache(self.block_id(start_block, block_dev.clone()), block_dev.clone())
                .lock()
                .write(0, |data_block: &mut DataBlock| {
                    let src = &buf[completed..completed + incr];
                    let dst = &mut data_block[start % BLOCK_SIZE..start % BLOCK_SIZE + incr];
                    dst.copy_from_slice(src);
                });

            completed += incr;
            start += incr;
            start_block += 1;
        }

        completed
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum InodeType {
    Invalid,
    File,
    Directory,
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    #[test]
    fn super_block_test() {
        let x = &mut [0u8; size_of::<SuperBlock>()];
        let sb = x as *mut _ as *mut SuperBlock;

        assert_eq!(
            unsafe { *sb },
            SuperBlock {
                magic:            0,
                size:             0,
                blocks_num:       0,
                inodes_num:       0,
                inode_bmap_start: 0,
                inode_start:      0,
                data_bmap_start:  0,
                data_start:       0,
            }
        );
        assert_eq!(unsafe { (*sb).is_valid() }, false);

        unsafe { (*sb).magic = FS_MAGIC }
        assert_eq!(unsafe { (*sb).is_valid() }, true);
    }

    #[test]
    fn bitmap_test() {
        let mut bmap = BitmapBlock([false; BLOCK_SIZE]);
        for _ in 0..BLOCK_SIZE {
            assert_eq!(bmap.allocate(), Some(0));
            bmap.free(0);
        }

        for i in 0..BLOCK_SIZE {
            assert_eq!(bmap.allocate(), Some(i));
        }

        for i in (0..BLOCK_SIZE).rev() {
            bmap.free(i);
        }
    }

    #[test]
    fn dir_entry_test() {
        for name in ["test", &"1".repeat(DIR_NAME_SIZE), "😀"] {
            let dirent = DirEntry::new(name, 2);
            assert_eq!(dirent.name(), name);
        }
    }

    #[test]
    fn dinode_test() {
        let x = &mut [0u8; size_of::<DInode>()];
        let inode = x as *mut _ as *mut DInode;

        assert_eq!(unsafe { (*inode).is_valid() }, false);
    }
}
