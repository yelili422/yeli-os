use core::mem::size_of;

use alloc::{collections::VecDeque, sync::Arc};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::block_dev::{BlockDevice, BlockId, InBlockOffset, BLOCK_SIZE};

/// The size of cache buffer.
const BLOCK_BUFFER_SIZE: usize = 64;

pub struct BlockCache {
    cache:     [u8; BLOCK_SIZE],
    block_id:  BlockId,
    block_dev: Arc<dyn BlockDevice>,
    modified:  bool,
}

impl BlockCache {
    /// Loads a new block from disk.
    pub fn new(block_id: BlockId, block_dev: Arc<dyn BlockDevice>) -> Self {
        let mut cache = [0u8; BLOCK_SIZE];
        block_dev.read(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_dev,
            modified: false,
        }
    }

    fn get_addr(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }

    pub unsafe fn get_ref<T>(&self, offset: InBlockOffset) -> &T
    where
        T: Sized,
    {
        let offset = offset as usize;
        let size = size_of::<T>();
        assert!(offset + size <= BLOCK_SIZE);

        &*(self.get_addr(offset) as *const T)
    }

    pub unsafe fn get_mut<T>(&mut self, offset: InBlockOffset) -> &mut T
    where
        T: Sized,
    {
        let offset = offset as usize;
        let size = size_of::<T>();
        assert!(offset + size <= BLOCK_SIZE);

        self.modified = true;
        &mut *(self.get_addr(offset) as *mut T)
    }

    pub fn read<T, V>(&self, offset: InBlockOffset, cb: impl FnOnce(&T) -> V) -> V {
        unsafe { cb(self.get_ref(offset)) }
    }

    pub fn write<T, V>(&mut self, offset: InBlockOffset, cb: impl FnOnce(&mut T) -> V) -> V {
        unsafe { cb(self.get_mut(offset)) }
    }

    /// Synchronize the cache back to disk.
    pub fn flush(&mut self) {
        if !self.modified {
            return;
        }

        self.modified = false;
        self.block_dev.write(self.block_id, &self.cache);
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.flush();
    }
}

pub struct BlockCacheBuffer {
    buffer: VecDeque<(BlockId, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheBuffer {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    pub fn get(
        &mut self,
        block_id: BlockId,
        block_dev: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some((_, cache)) = self.buffer.iter().find(|&&(bid, _)| bid == block_id) {
            cache.clone()
        } else {
            // Not cached.
            // Recycle the unused buffer by LRU.
            if self.buffer.len() == BLOCK_BUFFER_SIZE {
                // front to back.
                if let Some((idx, _)) = self
                    .buffer
                    .iter()
                    .enumerate()
                    .find(|(_, (_, cache))| Arc::strong_count(cache) == 1)
                {
                    self.buffer.remove(idx);
                } else {
                    // All buffers are busy, then too many processes are
                    // simultaneously executing file system calls.
                    // TODO: A more graceful response might to sleep until
                    // a buffer became free, though there would then be
                    // a possibility of deadlock.
                    panic!("Out of block cache buffer.");
                }
            }

            let block = Arc::new(Mutex::new(BlockCache::new(block_id, block_dev.clone())));
            self.buffer.push_back((block_id, block.clone()));

            block
        }
    }

    pub fn flush_all(&mut self) {
        for (_, cache) in self.buffer.iter() {
            cache.lock().flush()
        }
    }
}

lazy_static! {
    /// Linked list of all buffers. Sorted by how recently the buffer used.
    pub static ref BLOCK_CACHE_BUFFER: Mutex<BlockCacheBuffer> =
        Mutex::new(BlockCacheBuffer::new());
}

/// Gets block in buffer cache by block id.
///
/// Provides the basic access capability for block devices.
pub fn block_cache(block_id: BlockId, block_dev: Arc<dyn BlockDevice>) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_BUFFER.lock().get(block_id, block_dev)
}

pub fn flush() {
    BLOCK_CACHE_BUFFER.lock().flush_all();
}
