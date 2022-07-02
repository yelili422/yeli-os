mod stack;

use self::stack::StackFrameAllocator;
use crate::mem::{MEMORY_END, page::{Frame, PhysicalAddress}};
use lazy_static::lazy_static;
use spin::Mutex;

pub trait FrameAllocator {
    fn allocate(&mut self) -> Option<Frame>;
    fn free(&mut self, frame: &Frame);
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
    FRAME_ALLOCATOR.lock().allocate()
}

pub fn frame_deallocate(frame: &Frame) {
    FRAME_ALLOCATOR.lock().free(frame);
}
