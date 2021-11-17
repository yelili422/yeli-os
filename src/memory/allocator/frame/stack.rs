use super::FrameAllocator;
use crate::memory::page::PhysicalPageNum;
use alloc::vec::Vec;
use log::{info, trace};

#[derive(Debug)]
pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    pub fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    pub fn init(&mut self, start: PhysicalPageNum, end: PhysicalPageNum) {
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
    fn allocate(&mut self) -> Option<PhysicalPageNum> {
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
