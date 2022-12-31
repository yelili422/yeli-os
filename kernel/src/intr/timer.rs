use log::trace;
use riscv::register::time;

use crate::syscall::set_timer;

pub const INTERVAL: usize = 100_000;

pub static mut TICKS: usize = 0;

pub fn set_next_timer() {
    set_timer(time::read() + INTERVAL);
}

pub fn tick() {
    set_next_timer();
    unsafe {
        TICKS += 1;
        if TICKS % 100 == 0 {
            trace!("ticks: {}", TICKS);
        }
    }
}
