use super::address::{PhysicalAddress, PhysicalPageNum};
use super::config::MEMORY_END;
use alloc::vec::Vec;
use core::panic;
use lazy_static::lazy_static;
use log::{info, trace};
use spin::Mutex;

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysicalPageNum>;
    fn free(&mut self, ppn: PhysicalPageNum);
}

#[derive(Debug)]
pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    fn init(&mut self, start: PhysicalPageNum, end: PhysicalPageNum) {
        info!(
            "Init the frame allocator from {:?} to {:?}...",
            &start, &end
        );
        self.current = start.into();
        self.end = end.into();
        assert!(
            self.current < self.end,
            "No space has be allocated: {:?}",
            &self
        );
    }
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    fn alloc(&mut self) -> Option<PhysicalPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        } else {
            if self.current == self.end {
                None
            } else {
                self.current += 1;
                Some((self.current - 1).into())
            }
        }
    }

    fn free(&mut self, ppn: PhysicalPageNum) {
        let ppn = ppn.0;
        trace!("frame free:{} {}", &self.current, &ppn);
        if ppn >= self.current || self.recycled.iter().find(|&v| *v == ppn).is_some() {
            panic!("Frame ppn={:#x} has not been allocated.", ppn);
        }
        self.recycled.push(ppn);
        trace!("free ppn: {:x}", &ppn);
    }
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<FrameAllocatorImpl> =
        Mutex::new(FrameAllocatorImpl::new());
}

pub fn init() {
    extern "C" {
        fn kernel_end();
    }
    FRAME_ALLOCATOR.lock().init(
        PhysicalAddress::from(kernel_end as usize).ceil_page(),
        PhysicalAddress::from(MEMORY_END).floor_page(),
    );
}

/// [`FrameTracker`] can be interpreted as [`Box`](alloc::boxed::Box).
/// The deference is that the space is not allocated on the heap,
/// but in a physical page.
#[derive(Debug)]
pub struct FrameTracker {
    pub ppn: PhysicalPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysicalPageNum) -> Self {
        // page cleaning
        let bytes_array = ppn.get_bytes_array();
        for i in bytes_array {
            *i = 0;
        }
        trace!("Init physical page number: {:?}.", &ppn);
        Self { ppn }
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        trace!("drop: {:?}", &self);
        frame_free(self.ppn);
    }
}

pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .lock()
        .alloc()
        .map(|ppn| FrameTracker::new(ppn))
}

fn frame_free(ppn: PhysicalPageNum) {
    FRAME_ALLOCATOR.lock().free(ppn);
}

#[cfg(test)]
mod tests {
    use super::frame_alloc;

    #[test_case]
    fn test_success() {
        assert_eq!(1, 1);
    }

    #[test_case]
    fn test_frame_allocate() {
        assert!(frame_alloc().is_some());
    }
}
