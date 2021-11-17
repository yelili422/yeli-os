mod allocator;
mod heap;
mod page;
mod segment;

pub const MEMORY_END: usize = 0x8080_0000;

pub fn init() {
    heap::init();
    allocator::init();
    segment::init();
}
