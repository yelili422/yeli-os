use core::{
    alloc::{GlobalAlloc, Layout},
    ops::Deref,
    panic,
    ptr::null_mut,
};

use log::{info, trace};
use spin::Mutex;

use crate::mem::{
    address::{pa_as_mut, PhysAddr},
    allocator::list_allocator::ListAllocator,
    page::{PageSize, Size4KiB},
};

pub mod list_allocator;

pub trait Allocator {
    fn free(&mut self, pa: PhysAddr);
    fn alloc(&mut self) -> Option<PhysAddr>;
}

pub struct GlobalAllocator {
    inner: Mutex<Option<ListAllocator>>,
}

impl GlobalAllocator {
    const fn new() -> Self {
        GlobalAllocator {
            inner: Mutex::new(None),
        }
    }

    pub fn init(&mut self, pa_start: PhysAddr, pa_end: PhysAddr) {
        let mut allocator = ListAllocator::new(pa_start, pa_end);

        info!("Init allocator: {}", &allocator);
        allocator.free_range();
        *self.lock() = Some(allocator);
    }
}

// TODO: this is a temporary implement.
unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        trace!("allocate: {:?}", layout);
        let size = layout.size() as u64;

        if size > Size4KiB::SIZE {
            return null_mut();
        }

        match *self.lock() {
            Some(ref mut allocator) => {
                if let Some(page) = allocator.alloc() {
                    pa_as_mut(page)
                } else {
                    null_mut()
                }
            }
            _ => panic!(""),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        trace!("dealloc: {:?}", layout);

        match *self.lock() {
            Some(ref mut allocator) => {
                allocator.free(ptr as u64);
            }
            _ => panic!(""),
        }
    }
}

impl Deref for GlobalAllocator {
    type Target = Mutex<Option<ListAllocator>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
pub enum MallocErr {
    NotEnoughMemory,
}

#[global_allocator]
pub static mut FRAME_ALLOCATOR: GlobalAllocator = GlobalAllocator::new();

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

pub fn allocate() -> Result<PhysAddr, MallocErr> {
    unsafe {
        match *FRAME_ALLOCATOR.lock() {
            Some(ref mut allocator) => match allocator.alloc() {
                Some(page) => Ok(page),
                _ => Err(MallocErr::NotEnoughMemory),
            },
            _ => panic!(),
        }
    }
}

pub fn free(address: PhysAddr) {
    unsafe {
        if let Some(ref mut allocator) = *FRAME_ALLOCATOR.lock() {
            allocator.free(address);
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, vec::Vec};

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
    }
}
