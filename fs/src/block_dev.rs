use core::mem::size_of;

use alloc::sync::Arc;
use log::debug;
use spin::Mutex;

use crate::block_cache::BlockCacheBuffer;

/// The trait of block devices.
///
/// Blocks devices only support random read and write by block.
pub trait BlockDevice: Send + Sync {
    fn read(&self, block_id: u64, buf: &mut [u8]);
    fn write(&self, block_id: u64, buf: &[u8]);
}

/// The size of one block.
///
/// The smallest addressable unit on a block device is a *sector*.
/// Sectors come in various powers of two, but 512 bytes is the most
/// common size. Therefore, the block size can be no smaller than
/// the sector and must be a multiple of a sector. Furthermore,
/// the kernel needs the block to be a power of two. The kernel
/// also requires that a block be no larger than the page size.
/// Common block sizes are 512 bytes, 1 kilobyte, and 4 kilobytes.
pub const BLOCK_SIZE: usize = 4096; // Bytes

/// File system magic number for sanity check.
const FS_MAGIC: u64 = 0x102030;

/// Inode number in one block.
pub const INODES_PER_BLOCK: usize = BLOCK_SIZE / DINODE_SIZE;

/// Bitmap number in one block.
pub const BITMAP_PER_BLOCK: usize = BLOCK_SIZE * 8;

/// Direct blocks per inode.
///
/// We should keep every `DInode` to take up the most of space in
/// 1/n of `BLOCK_SIZE` preferably.
/// (i.e. DINODE_SIZE == BLOCK_SIZE / n)
pub const N_DIRECT: usize = 28;

/// Indirect blocks per block.
pub const N_INDIRECT: usize = BLOCK_SIZE / size_of::<BlockId>();

/// The maximum data blocks of one inode.
pub const MAX_BLOCKS_PER_INODE: usize = N_DIRECT + N_INDIRECT;

/// The maximum inode capacity.
pub const CAPACITY_PER_INODE: usize = MAX_BLOCKS_PER_INODE * BLOCK_SIZE;

/// The size of directory name.
pub const DIR_NAME_SIZE: usize = 24;

/// The size of directory entry.
pub const DIR_ENTRY_SIZE: usize = size_of::<DirEntry>();

/// The size of DInode.
pub const DINODE_SIZE: usize = size_of::<DInode>();

/// The maximum directories per inode.
pub const MAX_DIRENTS_PER_INODE: usize = CAPACITY_PER_INODE / DIR_ENTRY_SIZE;

/// The Inode ID.
///
/// Every inode is the same size, so it is easy, given a number n, to find
/// the nth inode on the disk. In fact, this number n, called the inode
/// number or i-number, is how inodes are identified in the implementation.
pub type InodeId = u64;

/// The block ID.
pub type BlockId = u64;

/// The block offset.
pub type InBlockOffset = u64;

/// Contains metadata about the file system.
///
/// Disk layout:
/// [ boot block | super block | inode bit map | inode blocks
///                               | data bit map | data blocks ]
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SuperBlock {
    /// Must be `FS_MAGIC`
    magic:                u64,
    /// Size of file system image (blocks).
    pub blocks:           u64,
    /// Block number of first free inode map block.
    pub inode_bmap_start: InodeId,
    /// Block number of first inode block.
    pub inode_start:      InodeId,
    /// Number of inodes.
    pub inode_blocks:     u64,
    /// Block number of first free data map block.
    pub data_bmap_start:  InodeId,
    /// Block number of first data block.
    pub data_start:       InodeId,
    /// Number of data blocks.
    pub data_blocks:      u64,
}

impl SuperBlock {
    pub fn new(
        blocks: u64,
        inode_bmap_start: InodeId,
        inode_start: InodeId,
        inode_blocks: u64,
        data_bmap_start: InodeId,
        data_start: InodeId,
        data_blocks: u64,
    ) -> SuperBlock {
        Self {
            magic: FS_MAGIC,
            blocks,
            inode_bmap_start,
            inode_start,
            inode_blocks,
            data_bmap_start,
            data_start,
            data_blocks,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == FS_MAGIC
    }

    /// Gets block id and offset-in-block by inode-num.
    pub fn find_inode(&self, inum: InodeId) -> (BlockId, InBlockOffset) {
        let block_id = inum / INODES_PER_BLOCK as u64 + self.inode_start;
        let offset = (inum % INODES_PER_BLOCK as u64) * DINODE_SIZE as u64;
        (block_id, offset)
    }
}

/// The type of bitmap block, group of `BLOCK_SIZE`.
#[repr(transparent)]
pub struct BitmapBlock {
    inner: [u8; BLOCK_SIZE],
}

impl BitmapBlock {
    pub fn allocate(&mut self) -> Option<usize> {
        for (i, &byte) in self.inner.iter().enumerate() {
            if byte == 0xff {
                continue;
            }
            for offset in 0..8 {
                if byte & (1 << offset) == 0 {
                    self.inner[i] |= 1 << offset;
                    return Some(i * 8 + offset);
                }
            }
        }
        None
    }

    pub fn free(&mut self, idx: usize) {
        let byte = idx / 8;
        let offset = idx % 8;
        assert_ne!(
            self.inner[byte] & (1 << offset),
            0,
            "bitmap: This bit is already freed. {}",
            idx
        );
        self.inner[byte] &= !(1 << offset);
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
/// It records the data block addresses of the file. The first N_DIRECT
/// blocks will be stored in `addresses`, and the rest will be stored in
/// the indirect blocks pointed by `indirect`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DInode {
    /// File type.
    pub type_:     InodeType,
    /// Indirect block number.
    pub indirect:  InodeId,
    /// Counts the number of directory entries that refer to this inode.
    pub links_num: u64,
    /// Size of file (bytes).
    pub size:      u64,
    /// Data block addresses.
    pub addresses: [BlockId; N_DIRECT],
}

impl DInode {
    pub fn new(
        type_: InodeType,
        indirect: InodeId,
        links_num: u64,
        size: u64,
        addresses: [BlockId; N_DIRECT],
    ) -> Self {
        Self {
            type_,
            indirect,
            links_num,
            size,
            addresses,
        }
    }

    pub fn initialize(&mut self, type_: InodeType) {
        *self = Self {
            type_,
            indirect: 0,
            links_num: 0,
            size: 0,
            addresses: [0; N_DIRECT],
        }
    }

    pub fn is_valid(&self) -> bool {
        self.type_ != InodeType::Invalid
    }

    /// Gets block id by inner index.
    pub fn get_bid(
        &self,
        idx: usize,
        block_dev: Arc<dyn BlockDevice>,
        cache: Arc<Mutex<BlockCacheBuffer>>,
    ) -> BlockId {
        assert!(idx < MAX_BLOCKS_PER_INODE);

        if idx < N_DIRECT {
            self.addresses[idx]
        } else if idx < N_DIRECT + N_INDIRECT {
            cache
                .lock()
                .get(self.indirect, block_dev.clone())
                .lock()
                .read(0, |index_block: &IndexBlock| index_block[idx - N_DIRECT])
        } else {
            panic!("the block index is out of range: {}", idx)
        }
    }

    /// Sets block id to given inner index.
    pub fn set_bid(
        &mut self,
        idx: usize,
        block_id: BlockId,
        block_dev: Arc<dyn BlockDevice>,
        cache: Arc<Mutex<BlockCacheBuffer>>,
    ) {
        assert!(idx < MAX_BLOCKS_PER_INODE);
        debug!("dinode: map idx: {} to block id: {}", idx, block_id);

        if idx < N_DIRECT {
            self.addresses[idx] = block_id;
        } else if idx < N_DIRECT + N_INDIRECT {
            cache
                .lock()
                .get(self.indirect, block_dev.clone())
                .lock()
                .write(0, |index_block: &mut IndexBlock| index_block[idx - N_DIRECT] = block_id)
        } else {
            panic!("the block index is out of range: {}", idx)
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
        cache: Arc<Mutex<BlockCacheBuffer>>,
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

            cache
                .lock()
                .get(self.get_bid(start_block, block_dev.clone(), cache.clone()), block_dev.clone())
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
    pub fn write_data(
        &self,
        offset: usize,
        buf: &[u8],
        block_dev: Arc<dyn BlockDevice>,
        cache: Arc<Mutex<BlockCacheBuffer>>,
    ) -> usize {
        let mut start_addr = offset;
        // Ensure the end address does not exceed the safe range.
        let end_addr = start_addr + buf.len().min(self.size as usize - offset);

        let mut start_block = start_addr / BLOCK_SIZE;
        let mut completed = 0usize;
        while start_addr < end_addr {
            // Growth value is the minimum of the end address or the block boundary.
            let incr = end_addr.min((start_block + 1) * BLOCK_SIZE) - start_addr;
            let block_id = self.get_bid(start_block, block_dev.clone(), cache.clone());

            cache.lock().get(block_id, block_dev.clone()).lock().write(
                0,
                |data_block: &mut DataBlock| {
                    let src = &buf[completed..completed + incr];
                    let dst =
                        &mut data_block[start_addr % BLOCK_SIZE..start_addr % BLOCK_SIZE + incr];
                    dst.copy_from_slice(src);
                },
            );

            completed += incr;
            start_addr += incr;
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
    fn test_super_block() {
        let x = &mut [0u8; size_of::<SuperBlock>()];
        let sb = x as *mut _ as *mut SuperBlock;

        assert_eq!(
            unsafe { *sb },
            SuperBlock {
                magic:            0,
                blocks:           0,
                data_blocks:      0,
                inode_blocks:     0,
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
    fn test_bitmap_size() {
        assert_eq!(size_of::<BitmapBlock>(), BLOCK_SIZE);
    }

    #[test]
    fn test_bitmap() {
        let mut bmap = BitmapBlock {
            inner: [0; BLOCK_SIZE],
        };

        for i in 0..BLOCK_SIZE * 8 {
            assert_eq!(bmap.allocate(), Some(i));
        }

        for i in (0..BLOCK_SIZE * 8).rev() {
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
