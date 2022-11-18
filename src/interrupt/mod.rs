mod context;
mod handler;
mod timer;

pub unsafe fn init() {
    handler::init();
    timer::init();
}
