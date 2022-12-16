// Almost all code copied from crate [buddyalloc](https://github.com/DrChat/buddyalloc/blob/master/src/heap.rs).

use core::{
    alloc::Layout,
    mem::size_of,
    ptr::{null_mut, NonNull},
};

use log::{debug, trace};

use crate::{
    is_aligned,
    mem::{allocator::AllocationError, PAGE_SIZE},
    memset,
};

macro_rules! max {
    ($x:expr) => ($x);
    ($x:expr, $($y:expr),+) => (
        core::cmp::max($x, max!($($y),+))
    )
}

struct FreeBlock {
    next: *mut FreeBlock,
}

impl FreeBlock {
    const fn new(next: *mut FreeBlock) -> FreeBlock {
        FreeBlock { next }
    }
}

pub struct BuddyAllocator<const ORDER: usize> {
    base: *mut u8,
    size: usize,
    free_list: [*mut FreeBlock; ORDER],
    min_block_size: usize,
}

impl<const ORDER: usize> BuddyAllocator<ORDER> {
    pub unsafe fn new(heap_base: NonNull<u8>, heap_end: NonNull<u8>) -> Self {
        let base_addr = heap_base.as_ptr() as u64;
        assert!(is_aligned!(base_addr, PAGE_SIZE), "The heap required 4k alignment.");

        let mut heap_size = heap_end.as_ptr() as usize - heap_base.as_ptr() as usize;
        if !heap_size.is_power_of_two() {
            heap_size = heap_size.next_power_of_two() >> 1;
        }
        let min_block_size = heap_size >> (ORDER - 1);

        debug!(
            "buddy_allocator: init from 0x{:x} to 0x{:x}, size: {}B",
            heap_base.as_ptr() as u64,
            heap_end.as_ptr() as u64,
            heap_size
        );
        debug!("buddy_allocator: order: {}, the minimum block size: {}", ORDER, min_block_size);

        assert!(
            min_block_size >= size_of::<FreeBlock>(),
            "The minimum block must be git enough to contain one block."
        );

        memset!(heap_base.as_ptr() as u64, 1, heap_size);

        let mut free_list = [null_mut(); ORDER];
        free_list[ORDER - 1] = heap_base.as_ptr() as *mut FreeBlock;

        Self {
            base: heap_base.as_ptr(),
            size: heap_size,
            free_list,
            min_block_size,
        }
    }

    unsafe fn split_free_block(&mut self, block: *mut u8, mut order: usize, order_needed: usize) {
        let mut size_to_split = 1 << (log2(self.min_block_size) + order);
        while order > order_needed {
            size_to_split >>= 1;
            order -= 1;

            let half = block.add(size_to_split);
            self.free_list_insert(order, half);
        }
    }

    fn free_list_pop(&mut self, order: usize) -> Option<*mut u8> {
        let candidate = self.free_list[order];
        if candidate.is_null() {
            None
        } else {
            self.free_list[order] = unsafe { (*candidate).next };
            Some(candidate as *mut u8)
        }
    }

    fn free_list_insert(&mut self, order: usize, block: *mut u8) {
        let free_block = block as *mut FreeBlock;
        unsafe { *free_block = FreeBlock::new(self.free_list[order]) };
        self.free_list[order] = free_block;
    }

    pub fn allocate(&mut self, layout: Layout) -> Result<*mut u8, AllocationError> {
        let mut align = layout.align();
        if !align.is_power_of_two() {
            align = align.next_power_of_two();
        }
        assert!(layout.align() as u64 <= PAGE_SIZE);

        let size = max!(layout.size().next_power_of_two(), align, self.min_block_size);
        let order_needed = log2(size) - log2(self.min_block_size);

        for order in order_needed..ORDER {
            if let Some(block) = self.free_list_pop(order) {
                if order > order_needed {
                    // Split this block to two smaller blockers,
                    // and append the second to the lower list.
                    unsafe { self.split_free_block(block, order, order_needed) };
                }

                memset!(block as u64, 0, size);
                trace!("--> alloc: 0x{:x}, size {}", block as u64, size);
                return Ok(block);
            }
        }

        Err(AllocationError::HeapExhausted)
    }

    pub fn free(&mut self, _ptr: NonNull<u8>, _layout: Layout) {
        // unimplemented!();
    }
}

fn log2(val: usize) -> usize {
    assert!(val.is_power_of_two());
    val.trailing_zeros() as usize
}

#[cfg(test)]
mod tests {
    use crate::{mem::MEM_END, pg_round_down};

    use super::*;

    #[test_case]
    fn test_allocate() {
        let base = pg_round_down!(MEM_END - 1024, PAGE_SIZE);

        unsafe {
            let mut allocator = BuddyAllocator::<3>::new(
                NonNull::new_unchecked((base + 0) as *mut _),
                NonNull::new_unchecked((base + 64) as *mut _),
            );
            let ptr = allocator
                .allocate(Layout::from_size_align_unchecked(16, 1))
                .unwrap();
            assert_eq!(ptr, base as *mut _);
        }
    }
}
