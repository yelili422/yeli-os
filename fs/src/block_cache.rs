use core::mem::size_of;

use alloc::{collections::VecDeque, sync::Arc};
use spin::Mutex;

use crate::block_dev::{BlockDevice, BlockId, InBlockOffset, BLOCK_SIZE};

/// The size of cache buffer.
pub const BLOCK_BUFFER_SIZE: usize = 64;

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

    pub fn clear(&mut self) {
        self.modified = true;
        self.cache.fill(0);
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
        assert!(offset + size <= BLOCK_SIZE, "offset: {}, size: {}", offset, size);

        &*(self.get_addr(offset) as *const T)
    }

    pub unsafe fn get_mut<T>(&mut self, offset: InBlockOffset) -> &mut T
    where
        T: Sized,
    {
        let offset = offset as usize;
        let size = size_of::<T>();
        assert!(offset + size <= BLOCK_SIZE, "offset: {}, size: {}", offset, size);

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
    pub fn sync(&mut self) {
        if !self.modified {
            return;
        }

        self.modified = false;
        self.block_dev.write(self.block_id, &self.cache);
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync();
    }
}

/// Linked list of all buffers. Sorted by how recently the buffer used.
pub struct BlockCacheBuffer {
    buffer:   VecDeque<(BlockId, Arc<Mutex<BlockCache>>)>,
    capacity: usize,
}

impl BlockCacheBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::new(),
            capacity,
        }
    }

    /// Look through buffer cache for block on device dev.
    /// If not found, allocate a buffer.
    /// In either case, return locked buffer.
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
            if self.buffer.len() == self.capacity {
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

    pub fn flush(&mut self) {
        for (_, cache) in self.buffer.iter() {
            cache.lock().sync()
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    #[allow(unused_imports)]
    use super::*;

    struct MockBlockDevice {
        pub data: [u8; BLOCK_SIZE],
    }

    impl MockBlockDevice {
        pub fn new() -> Self {
            Self {
                data: [0; BLOCK_SIZE],
            }
        }
    }

    impl BlockDevice for MockBlockDevice {
        fn read(&self, _block_id: BlockId, buf: &mut [u8])  -> Result<(), String>  {
            buf.copy_from_slice(&self.data);
            Ok(())
        }

        fn write(&self, _block_id: BlockId, _buf: &[u8]) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn test_block_cache_buffer() {
        let dev = Arc::new(MockBlockDevice::new());
        let mut block_cache = BlockCacheBuffer::new(2);

        let cache1 = block_cache.get(1, dev.clone());
        let cache2 = block_cache.get(2, dev.clone());

        assert_eq!(block_cache.buffer.len(), 2);
        assert_eq!(block_cache.buffer[0].0, 1);
        assert_eq!(block_cache.buffer[1].0, 2);

        drop(cache1);
        let cache3 = block_cache.get(3, dev.clone());
        assert_eq!(block_cache.buffer.len(), 2);
        assert_eq!(block_cache.buffer[0].0, 2);
        assert_eq!(block_cache.buffer[1].0, 3);

        drop(cache2);
        drop(cache3);
        assert_eq!(block_cache.buffer.len(), 2);
        assert_eq!(block_cache.buffer[0].0, 2);
        assert_eq!(block_cache.buffer[1].0, 3);
    }
}
