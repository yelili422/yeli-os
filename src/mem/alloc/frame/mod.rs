mod stack;

use self::stack::StackFrameAllocator;
use crate::mem::page::PhysicalPageNum;
use lazy_static::lazy_static;
use spin::Mutex;

pub trait FrameAllocator {
    fn allocate(&mut self) -> Option<PhysicalPageNum>;
    fn free(&mut self, ppn: &PhysicalPageNum);
}

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<StackFrameAllocator> =
        Mutex::new(StackFrameAllocator::new());
}

pub fn init(start: PhysicalPageNum, end: PhysicalPageNum) {
    FRAME_ALLOCATOR.lock().init(start, end);
}

pub fn frame_allocate() -> Option<PhysicalPageNum> {
    FRAME_ALLOCATOR.lock().allocate()
}

pub fn frame_deallocate(ppn: &PhysicalPageNum) {
    FRAME_ALLOCATOR.lock().free(ppn);
}
