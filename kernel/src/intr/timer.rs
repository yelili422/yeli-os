use core::sync::atomic::{AtomicUsize, Ordering};

use log::debug;
use riscv::register::time;

use crate::syscall::set_timer;

pub const INTERVAL: usize = 100_000;

pub static TICKS: AtomicUsize = AtomicUsize::new(0);

pub fn set_next_timer() {
    set_timer(time::read() + INTERVAL);
}

pub fn tick() {
    set_next_timer();
    TICKS.fetch_add(1, Ordering::Relaxed);
    if TICKS.load(Ordering::Relaxed) % 100 == 0 {
        debug!("ticks: {}", TICKS.load(Ordering::Relaxed));
    }
}
