mod stack;

use self::stack::StackFrameAllocator;
use crate::memory::{
    page::{Frame, PhysicalAddress, PhysicalPageNum},
    MEMORY_END,
};
use lazy_static::lazy_static;
use spin::Mutex;

pub trait FrameAllocator {
    fn allocate(&mut self) -> Option<PhysicalPageNum>;
    fn free(&mut self, ppn: PhysicalPageNum);
}

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<StackFrameAllocator> =
        Mutex::new(StackFrameAllocator::new());
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

pub fn frame_allocate() -> Option<Frame> {
    FRAME_ALLOCATOR.lock().allocate().map(|ppn| Frame::new(ppn))
}

pub fn frame_deallocate(ppn: PhysicalPageNum) {
    FRAME_ALLOCATOR.lock().free(ppn);
}
