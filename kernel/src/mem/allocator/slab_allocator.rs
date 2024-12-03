use super::FrameAllocator;
use crate::{mem::PAGE_SIZE, pg_round_up};
use core::ptr::NonNull;
use log::trace;
use spin::Mutex;

/// The number of pages used for a slab.
pub const SLAB_PAGES: usize = 2;

/// The maximum order supported by the slab allocator.
pub const MAX_SLAB_ORDER: usize = 12;

#[repr(C)]
struct FreeBlock {
    next: Option<NonNull<FreeBlock>>,
}

#[repr(C)]
struct SlabHeader {
    free_list:      Option<NonNull<FreeBlock>>,
    object_start:   NonNull<u8>,
    object_end:     NonNull<u8>,
    active_objects: usize,
    next:           Option<NonNull<SlabHeader>>,
}

impl SlabHeader {
    unsafe fn init(&mut self, object_start: NonNull<u8>, object_size: usize, total_objects: usize) {
        let mut free_list = None;

        for i in 0..total_objects {
            let obj_addr = object_start.as_ptr() as usize + i * object_size;
            let obj_ptr = obj_addr as *mut FreeBlock;
            (*obj_ptr).next = free_list;
            free_list = NonNull::new(obj_ptr);
        }

        self.free_list = free_list;
        self.next = None;
        self.active_objects = 0;
        self.object_start = object_start;
        self.object_end = object_start.add(object_size * total_objects);
    }

    pub fn alloc(&mut self) -> Option<NonNull<u8>> {
        self.free_list.map(|node| unsafe {
            self.free_list = (*node.as_ptr()).next;
            self.active_objects += 1;
            NonNull::new_unchecked(node.as_ptr() as *mut u8)
        })
    }

    pub fn free(&mut self, obj: NonNull<u8>) {
        let obj_ptr = obj.as_ptr() as *mut FreeBlock;
        unsafe {
            (*obj_ptr).next = self.free_list;
            self.active_objects -= 1;
            self.free_list = NonNull::new(obj_ptr);
        }
    }

    pub fn contains(&self, obj: NonNull<u8>) -> bool {
        obj.as_ptr() >= self.object_start.as_ptr() && obj.as_ptr() < self.object_end.as_ptr()
    }
}

pub struct MemCache {
    object_size: usize,
    align:       usize,
    slabs:       Option<NonNull<SlabHeader>>,
}

impl MemCache {
    pub const fn new(object_size: usize, align: usize) -> Self {
        assert!(align.is_power_of_two(), "align must be a power of two");
        assert!(object_size >= align, "object_size must be greater than or equal to align");
        assert!(object_size >= 8, "object_size must be greater than or equal to 8");
        assert!(align >= 8, "align must be greater than or equal to 8");
        Self {
            object_size,
            align,
            slabs: None,
        }
    }

    fn alloc_slab(&mut self, frame_allocator: &Mutex<dyn FrameAllocator>) -> Option<usize> {
        let mut frame_allocator = frame_allocator.lock();
        frame_allocator.alloc_pages(SLAB_PAGES).map(|page| {
            let slab_ptr = page as *mut SlabHeader;
            let object_start = pg_round_up!(page + size_of::<SlabHeader>(), self.align);
            let object_end = page + SLAB_PAGES * PAGE_SIZE;
            assert!(object_start < object_end, "object_start must less than object_end");
            trace!("object_start: 0x{:x}, object_end: 0x{:x}", object_start, object_end);
            unsafe {
                (*slab_ptr).init(
                    NonNull::new_unchecked(object_start as *mut u8),
                    self.object_size,
                    (object_end - object_start) / self.object_size,
                );

                (*slab_ptr).next = self.slabs;
                self.slabs = NonNull::new(slab_ptr);
            };
            page
        })
    }

    pub fn alloc(&mut self, frame_allocator: &Mutex<dyn FrameAllocator>) -> Option<NonNull<u8>> {
        loop {
            let mut current_slab = self.slabs;
            while current_slab.is_some() {
                let mut slab_ptr = current_slab.unwrap();
                unsafe {
                    let slab = slab_ptr.as_mut();
                    if slab.free_list.is_some() {
                        return slab.alloc();
                    }
                    current_slab = (*slab_ptr.as_ptr()).next;
                }
            }

            if self.alloc_slab(frame_allocator).is_none() {
                break;
            }
        }

        None
    }

    fn free_slab(
        &mut self,
        slab_ptr: NonNull<SlabHeader>,
        frame_allocator: &Mutex<dyn FrameAllocator>,
    ) {
        let mut frame_allocator = frame_allocator.lock();
        frame_allocator.free_pages(slab_ptr.as_ptr() as usize, SLAB_PAGES);

        unsafe {
            if self.slabs == Some(slab_ptr) {
                self.slabs = (*slab_ptr.as_ptr()).next;
                return;
            }

            let mut current_slab = self.slabs;
            while current_slab.is_some() {
                let slab_ptr = current_slab.unwrap();
                if (*slab_ptr.as_ptr()).next == Some(slab_ptr) {
                    (*slab_ptr.as_ptr()).next = match (*slab_ptr.as_ptr()).next {
                        Some(next) => (*next.as_ptr()).next,
                        None => None,
                    };
                    break;
                }
                current_slab = (*slab_ptr.as_ptr()).next;
            }
        }
    }

    pub fn free(&mut self, obj: NonNull<u8>, frame_allocator: &Mutex<dyn FrameAllocator>) {
        let mut current_slab = self.slabs;
        while current_slab.is_some() {
            let mut slab_ptr = current_slab.unwrap();
            unsafe {
                let slab = slab_ptr.as_mut();
                if slab.contains(obj) {
                    slab.free(obj);

                    if slab.active_objects == 0 {
                        self.free_slab(slab_ptr, frame_allocator);
                    }
                    return;
                }
                current_slab = (*slab_ptr.as_ptr()).next;
            }
        }
    }
}

pub struct SlabAllocator {
    caches:          [Mutex<MemCache>; MAX_SLAB_ORDER + 1],
    frame_allocator: &'static Mutex<dyn FrameAllocator>,
}

impl SlabAllocator {
    pub const fn new(frame_allocator: &'static Mutex<dyn FrameAllocator>) -> Self {
        Self {
            caches: [
                Mutex::new(MemCache::new(8, 8)),
                Mutex::new(MemCache::new(8, 8)),
                Mutex::new(MemCache::new(8, 8)),
                Mutex::new(MemCache::new(8, 8)),
                Mutex::new(MemCache::new(16, 8)),
                Mutex::new(MemCache::new(32, 8)),
                Mutex::new(MemCache::new(64, 8)),
                Mutex::new(MemCache::new(128, 8)),
                Mutex::new(MemCache::new(256, 8)),
                Mutex::new(MemCache::new(512, 8)),
                Mutex::new(MemCache::new(1024, 8)),
                Mutex::new(MemCache::new(2048, 8)),
                Mutex::new(MemCache::new(4096, 8)),
            ],
            frame_allocator,
        }
    }
}

impl SlabAllocator {
    pub fn alloc(&self, order: usize) -> Option<NonNull<u8>> {
        assert!(order <= MAX_SLAB_ORDER);
        self.caches[order].lock().alloc(self.frame_allocator)
    }

    pub fn free(&self, order: usize, obj: NonNull<u8>) {
        assert!(order <= MAX_SLAB_ORDER);
        self.caches[order].lock().free(obj, self.frame_allocator)
    }
}

unsafe impl Sync for SlabAllocator {}
unsafe impl Send for SlabAllocator {}

#[cfg(test)]
mod tests {
    use spin::mutex::Mutex;

    use crate::mem::allocator::buddy_allocator;

    use super::*;

    struct MockMemory {
        data: [u8; 4 * 1024 * 1024],
    }

    impl MockMemory {
        fn new() -> Self {
            let data = [0u8; 4 * 1024 * 1024];
            MockMemory { data }
        }

        fn start_addr(&self) -> usize {
            self.data.as_ptr() as usize
        }

        fn end_addr(&self) -> usize {
            self.data.as_ptr() as usize + self.data.len()
        }
    }

    #[test_case]
    fn test_slab_allocate() {
        let mock_mem = MockMemory::new();
        let buddy_allocator = Mutex::new(buddy_allocator::BuddyAllocator::new());
        buddy_allocator
            .lock()
            .init(mock_mem.start_addr(), mock_mem.end_addr());

        let mut mem_cache = MemCache::new(8, 8);
        let objects = (PAGE_SIZE * SLAB_PAGES - size_of::<SlabHeader>()) / 8 - 1;

        for _ in 0..objects {
            let obj = mem_cache.alloc(&buddy_allocator).unwrap();
            assert!(obj.as_ptr() as usize % 8 == 0);
        }
    }

    #[test_case]
    fn test_slab_free() {
        let mock_mem = MockMemory::new();
        let buddy_allocator = Mutex::new(buddy_allocator::BuddyAllocator::new());
        buddy_allocator
            .lock()
            .init(mock_mem.start_addr(), mock_mem.end_addr());

        let mut mem_cache = MemCache::new(8, 8);
        let objects = (PAGE_SIZE * SLAB_PAGES - size_of::<SlabHeader>()) / 8 - 1;

        for _ in 0..objects {
            let obj = mem_cache.alloc(&buddy_allocator).unwrap();
            mem_cache.free(obj, &buddy_allocator);
        }
    }
}
