use core::ptr::NonNull;

use log::{debug, error, info, trace};

use super::FrameAllocator;
use crate::{
    is_aligned,
    mem::{allocator::order, PAGE_SIZE},
    pg_round_down, pg_round_up,
};

/// Maximum order supported by the allocator.
/// 16 means 65536 pages (256 MiB for a 4K page size).
const MAX_ORDER: usize = 16;

struct FreeBlock {
    next: Option<NonNull<FreeBlock>>,
}

pub struct BuddyAllocator {
    /// Free address linked list.
    free_lists: [Option<NonNull<FreeBlock>>; MAX_ORDER],
    start_addr: usize,
    end_addr:   usize,
}

impl BuddyAllocator {
    pub const fn new() -> Self {
        Self {
            free_lists: [None; MAX_ORDER],
            start_addr: 0,
            end_addr:   0,
        }
    }

    /// Initialize the allocator with the given memory range.
    ///
    /// Rust global_allocator needs static function to be initialized. So, we
    /// split new and init into two functions.
    pub fn init(&mut self, start_addr: usize, end_addr: usize) {
        trace!("buddy_allocator: init from 0x{:x} to 0x{:x}", start_addr, end_addr);
        let start = pg_round_up!(start_addr, PAGE_SIZE);
        let end = pg_round_down!(end_addr, PAGE_SIZE);
        let pages = (end - start) / PAGE_SIZE;

        assert!(start < end, "start_addr must less than end_addr after align");
        self.start_addr = start;
        self.end_addr = end;

        let mut current_size = 1usize << (MAX_ORDER - 1);
        let mut addr = start;

        while addr < end {
            while current_size > pages || addr + current_size * PAGE_SIZE > end {
                current_size >>= 1;
            }
            if current_size == 0 {
                break;
            }

            let order = current_size.trailing_zeros() as usize;
            let block = addr as *mut FreeBlock;
            unsafe {
                (*block).next = self.free_lists[order];
                self.free_lists[order] = NonNull::new(block);
            }

            addr += current_size * PAGE_SIZE;
        }

        info!(
            "buddy_allocator: initialized. start_addr: 0x{:x}, end_addr: 0x{:x}, pages: {}",
            start, end, pages
        );
    }

    fn split_block(
        &mut self,
        block_order: usize,
        target_order: usize,
    ) -> Option<NonNull<FreeBlock>> {
        if block_order < target_order {
            return None;
        }

        let block = self.free_lists[block_order]?;
        self.free_lists[block_order] = unsafe { (*block.as_ptr()).next };

        // Split until reaching the target order.
        let mut current_order = block_order;
        while current_order > target_order {
            current_order -= 1;
            unsafe {
                let buddy =
                    (block.as_ptr() as usize + (1 << current_order) * PAGE_SIZE) as *mut FreeBlock;
                (*buddy).next = self.free_lists[current_order];
                self.free_lists[current_order] = NonNull::new(buddy);
            }
        }

        Some(block)
    }
}

impl FrameAllocator for BuddyAllocator {
    fn alloc_pages(&mut self, mut pages: usize) -> Option<usize> {
        if pages == 0 {
            return None;
        }

        if pages > (1 << (MAX_ORDER - 1)) {
            error!("buddy_allocator: alloc too many pages: {}", pages);
            return None;
        }

        pages = pages.next_power_of_two();

        let order = pages.trailing_zeros() as usize;
        let block_opt = (order..MAX_ORDER)
            .find(|&o| self.free_lists[o].is_some())
            .and_then(|o| self.split_block(o, order));

        block_opt.map(|block| {
            debug!(
                "buddy_allocator: alloc {} pages: 0x{:x} - 0x{:x}",
                pages,
                block.as_ptr() as usize,
                block.as_ptr() as usize + pages * PAGE_SIZE
            );
            block.as_ptr() as usize
        })
    }

    fn free_pages(&mut self, addr: usize, mut pages: usize) {
        debug!("buddy_allocator: dealloc {} pages from 0x{:x}", pages, addr);
        if pages == 0 || pages > (1 << (MAX_ORDER - 1)) {
            return;
        }

        if addr < self.start_addr || addr >= self.end_addr {
            return;
        }

        assert!(is_aligned!(addr, PAGE_SIZE), "addr must be page aligned");

        let mut order = order(pages);

        // 尝试合并伙伴块
        let mut block_addr = addr;
        while order < MAX_ORDER - 1 {
            // 计算伙伴块地址
            let buddy_addr =
                self.start_addr + ((block_addr - self.start_addr) ^ (pages * PAGE_SIZE));

            // 检查伙伴块是否在空闲链表中
            if let Some(mut current) = self.free_lists[order] {
                let mut found = false;
                let mut prev: Option<NonNull<FreeBlock>> = None;

                while let Some(curr) = NonNull::new(current.as_ptr()) {
                    if curr.as_ptr() as usize == buddy_addr {
                        // 找到伙伴块,从链表移除
                        found = true;
                        unsafe {
                            if let Some(p) = prev {
                                (*p.as_ptr()).next = (*curr.as_ptr()).next;
                            } else {
                                self.free_lists[order] = (*curr.as_ptr()).next;
                            }
                        }
                        break;
                    }
                    prev = Some(current);
                    match unsafe { (*current.as_ptr()).next } {
                        Some(next) => current = next,
                        None => break,
                    }
                }

                if !found {
                    break;
                }

                // 合并块
                block_addr = core::cmp::min(block_addr, buddy_addr);
                pages *= 2;
                order += 1;
            } else {
                break;
            }
        }

        // 将合并后的块加入空闲链表
        let block = block_addr as *mut FreeBlock;
        unsafe {
            (*block).next = self.free_lists[order];
            self.free_lists[order] = NonNull::new(block);
        }
    }
}

unsafe impl Sync for BuddyAllocator {}
unsafe impl Send for BuddyAllocator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C, align(4096))]
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
    fn test_new_allocator() {
        let mock_mem = MockMemory::new();
        let mut allocator = BuddyAllocator::new();
        allocator.init(mock_mem.start_addr(), mock_mem.end_addr());

        assert!(allocator.free_lists.iter().any(|list| list.is_some()));

        let addr1 = allocator.alloc_pages(1).unwrap();
        assert!(addr1 >= mock_mem.start_addr());
        assert!(addr1 < mock_mem.end_addr());
        assert_eq!(addr1 & (PAGE_SIZE - 1), 0);

        allocator.free_pages(addr1, 1);
    }

    #[test_case]
    fn test_multiple_allocs() {
        let mock_mem = MockMemory::new();
        let mut allocator = BuddyAllocator::new();
        allocator.init(mock_mem.start_addr(), mock_mem.end_addr());

        let addr1 = allocator.alloc_pages(1).unwrap();
        let addr2 = allocator.alloc_pages(2).unwrap();
        let addr4 = allocator.alloc_pages(4).unwrap();

        assert_eq!(addr1 & (PAGE_SIZE - 1), 0);
        assert_eq!(addr2 & (PAGE_SIZE - 1), 0);
        assert_eq!(addr4 & (PAGE_SIZE - 1), 0);

        // 验证地址在有效范围内
        assert!(addr1 >= mock_mem.start_addr() && addr1 < mock_mem.end_addr());
        assert!(addr2 >= mock_mem.start_addr() && addr2 < mock_mem.end_addr());
        assert!(addr4 >= mock_mem.start_addr() && addr4 < mock_mem.end_addr());

        allocator.free_pages(addr1, 1);
        allocator.free_pages(addr2, 2);
        allocator.free_pages(addr4, 4);
    }

    #[test_case]
    fn test_fragmentation_and_coalescing() {
        let mock_mem = MockMemory::new();
        let mut allocator = BuddyAllocator::new();
        allocator.init(mock_mem.start_addr(), mock_mem.end_addr());

        let addr1 = allocator.alloc_pages(1).unwrap();
        let addr2 = allocator.alloc_pages(1).unwrap();
        let addr3 = allocator.alloc_pages(2).unwrap();

        assert!(addr1 + 2 * PAGE_SIZE == addr3);

        allocator.free_pages(addr1, 1);
        allocator.free_pages(addr2, 1);

        let addr4 = allocator.alloc_pages(2).unwrap();

        assert_eq!(addr4, addr1);

        allocator.free_pages(addr4, 2);
        allocator.free_pages(addr3, 2);

        let addr5 = allocator.alloc_pages(4).unwrap();

        assert_eq!(addr5, addr1);
    }

    #[test_case]
    fn test_invalid_inputs() {
        let mock_mem = MockMemory::new();
        let mut allocator = BuddyAllocator::new();
        allocator.init(mock_mem.start_addr(), mock_mem.end_addr());

        assert!(allocator.alloc_pages(0).is_none());

        // 测试范围外和未对齐的地址
        allocator.free_pages(mock_mem.start_addr() - PAGE_SIZE, 1);
        // allocator.dealloc(mock_mem.start_addr() + 1, 1); // will panic
        allocator.free_pages(mock_mem.end_addr() + PAGE_SIZE, 1);

        allocator.free_pages(mock_mem.start_addr(), 0);
        allocator.free_pages(mock_mem.start_addr(), 1 << MAX_ORDER);
    }

    #[test_case]
    fn test_alignment_requirements() {
        let mock_mem = MockMemory::new();
        // 使用未对齐的起始地址
        let mut allocator = BuddyAllocator::new();
        allocator.init(mock_mem.start_addr(), mock_mem.end_addr());

        if let Some(addr) = allocator.alloc_pages(1) {
            assert!(is_aligned!(addr, PAGE_SIZE));
            allocator.free_pages(addr, 1);
        }
    }
}
