use crate::mem::{address::PhysicalAddress, PAGE_SIZE};
use buddy_allocator::BuddyAllocator;
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::{null_mut, NonNull},
};
use slab_allocator::{SlabAllocator, MAX_SLAB_ORDER};
use spin::Mutex;

mod buddy_allocator;
mod slab_allocator;

pub trait FrameAllocator {
    fn alloc_pages(&mut self, pages: usize) -> Option<PhysicalAddress>;
    fn free_pages(&mut self, addr: PhysicalAddress, pages: usize);
}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: size: {} bytes, align: {}", layout.size(), layout.align())
}

static FRAME_ALLOCATOR: Mutex<BuddyAllocator> = Mutex::new(BuddyAllocator::new());

static SLAB_ALLOCATOR: SlabAllocator = SlabAllocator::new(&FRAME_ALLOCATOR);

pub struct GlobalAllocator {}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let order = order(layout.size());
        if order > MAX_SLAB_ORDER {
            let pages = (layout.size() + (PAGE_SIZE - 1)) / PAGE_SIZE;
            FRAME_ALLOCATOR
                .lock()
                .alloc_pages(pages)
                .map(|addr| addr as *mut u8)
                .unwrap_or(null_mut())
        } else {
            SLAB_ALLOCATOR
                .alloc(order)
                .map(|ptr| ptr.as_ptr())
                .unwrap_or(null_mut())
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let order = order(layout.size());
        if order > MAX_SLAB_ORDER {
            let pages = (layout.size() + (PAGE_SIZE - 1)) / PAGE_SIZE;
            FRAME_ALLOCATOR
                .lock()
                .free_pages(ptr as PhysicalAddress, pages);
        } else {
            SLAB_ALLOCATOR.free(order, NonNull::new_unchecked(ptr));
        }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator {};

pub unsafe fn init_allocator(mem_start: PhysicalAddress, mem_end: PhysicalAddress) {
    FRAME_ALLOCATOR.lock().init(mem_start, mem_end);
}

pub fn alloc_pages(pages: usize) -> Option<PhysicalAddress> {
    FRAME_ALLOCATOR.lock().alloc_pages(pages)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllocationError {
    HeapExhausted,
    InvalidSize,
}

pub fn order(size: usize) -> usize {
    size.next_power_of_two().trailing_zeros() as usize
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, vec, vec::Vec};

    #[test_case]
    fn test_heap_alloc() {
        let a = Box::new(42);
        assert_eq!(*a, 42);
        drop(a);

        let mut v: Vec<usize> = Vec::new();
        for i in 0..500 {
            v.push(i);
        }
        for (i, val) in v.iter().take(500).enumerate() {
            assert_eq!(*val, i);
        }

        let mut p = vec![0; 2 * 4096].into_boxed_slice();
        for i in p.iter_mut() {
            *i = 5;
            assert_eq!(*i, 5);
        }
    }
}
