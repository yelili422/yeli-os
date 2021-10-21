mod address;
mod config;
mod frame;
mod heap;
mod page;
mod segment;

pub fn init() {
    heap::init();
    frame::init();
    segment::init();
}
