use crate::mem::{address::PhysicalAddress, PAGE_SIZE};
pub use buddy_allocator::FRAME_ALLOCATOR;
use core::alloc::{GlobalAlloc, Layout};

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

pub fn alloc_one_page() -> Option<PhysicalAddress> {
    let page =
        unsafe { FRAME_ALLOCATOR.alloc(Layout::from_size_align_unchecked(PAGE_SIZE, PAGE_SIZE)) };

    if page.is_null() {
        None
    } else {
        Some(page as usize)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllocationError {
    HeapExhausted,
    InvalidSize,
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
