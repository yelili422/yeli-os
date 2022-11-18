use crate::syscall::set_timer;
use log::trace;
use riscv::register::{sie, sstatus, time};

pub static mut TICKS: usize = 0;

static INTERVAL: usize = 100000;

pub unsafe fn init() {
    // enable timer interrupt
    sie::set_stimer();
    sstatus::set_sie();
    set_next_timer();
}

fn set_next_timer() {
    set_timer(time::read() + INTERVAL);
}

pub fn tick() {
    set_next_timer();
    unsafe {
        TICKS += 1;
        if TICKS % 100 == 0 {
            trace!("{} tick", TICKS);
        }
    }
}
