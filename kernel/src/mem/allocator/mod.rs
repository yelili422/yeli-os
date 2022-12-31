use core::{
    alloc::{GlobalAlloc, Layout},
    ops::Deref,
    panic,
    ptr::{null_mut, NonNull},
};

use log::{debug, error};
use spin::Mutex;

use crate::{
    mem::{
        address::{as_mut, PhysicalAddress},
        PAGE_SIZE,
    },
    pa2va, va2pa,
};

use self::buddy_allocator::BuddyAllocator;

mod buddy_allocator;
mod bump_allocator;
// mod list_allocator;

pub trait FrameAllocator {
    fn allocate(&mut self) -> Option<PhysicalAddress>;
    fn free(&mut self, pa: PhysicalAddress);
}

pub struct LockedAllocator<const ORDER: usize> {
    inner: Mutex<Option<BuddyAllocator<ORDER>>>,
}

impl<const ORDER: usize> LockedAllocator<ORDER> {
    const fn new() -> Self {
        LockedAllocator {
            inner: Mutex::new(None),
        }
    }

    pub unsafe fn init(&mut self, pa_start: PhysicalAddress, pa_end: PhysicalAddress) {
        debug!("locked_allocator: init from 0x{:x} to 0x{:x}", pa_start, pa_end);
        let mut allocator = self.lock();
        {
            *allocator = Some(BuddyAllocator::<ORDER>::new(
                NonNull::new_unchecked(pa_start as *mut _),
                NonNull::new_unchecked(pa_end as *mut _),
            ))
        }
    }
}

unsafe impl<const ORDER: usize> GlobalAlloc for LockedAllocator<ORDER> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        debug!("allocator: allocate {:?}", layout);

        match *self.lock() {
            Some(ref mut allocator) => match allocator.allocate(layout) {
                Ok(pa) => as_mut(pa2va!(pa as usize)),
                Err(_) => {
                    error!("allocate finished, but return a null pointer.");
                    null_mut()
                }
            },
            _ => panic!(""),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        debug!("dealloc: {:?}", layout);

        match *self.lock() {
            Some(ref mut allocator) => {
                allocator.free(NonNull::new_unchecked(va2pa!(ptr as usize) as *mut _), layout);
            }
            _ => panic!(""),
        }
    }
}

impl<const B: usize> Deref for LockedAllocator<B> {
    type Target = Mutex<Option<BuddyAllocator<B>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[global_allocator]
pub static mut FRAME_ALLOCATOR: LockedAllocator<16> = LockedAllocator::<16>::new();

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: size: {}, align: {}", layout.size(), layout.align())
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
