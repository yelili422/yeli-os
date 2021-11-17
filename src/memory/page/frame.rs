use super::PhysicalPageNum;
use crate::memory::allocator::frame_deallocate;
use log::trace;

/// [`Frame`] can be interpreted as [`Box`](alloc::boxed::Box).
/// The deference is that the space is not allocated on the heap,
/// but in a physical page.
#[derive(Debug)]
pub struct Frame {
    ppn: PhysicalPageNum,
}

impl Frame {
    pub fn from(ppn: PhysicalPageNum) -> Self {
        Self { ppn }
    }

    pub fn create(ppn: PhysicalPageNum) -> Self {
        // page cleaning
        let bytes_array = ppn.get_bytes_array();
        for i in bytes_array {
            *i = 0;
        }
        trace!("Init physical page number: {:?}.", &ppn);
        Self { ppn }
    }

    pub fn ppn(&self) -> PhysicalPageNum {
        self.ppn
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        trace!("drop: {:?}", &self);
        frame_deallocate(self);
    }
}

#[cfg(test)]
mod tests {
    use crate::memory::allocator::frame_allocate;

    #[test_case]
    fn test_success() {
        assert_eq!(1, 1);
    }

    #[test_case]
    fn test_frame_allocate() {
        assert!(frame_allocate().is_some());
    }
}
