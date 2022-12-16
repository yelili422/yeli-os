#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(lang_items::test_runner)]
// The custom test frameworks feature generates a main function that
// calls test_runner, but this function is ignored because we use
// the #[no_main] attribute and provide our own entry point.
#![reexport_test_harness_main = "test_main"]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(step_trait)]

extern crate alloc;

use core::arch::global_asm;
use log::{info, LevelFilter};

pub mod console;
pub mod interrupt;
pub mod lang_items;
pub mod logger;
pub mod mem;
pub mod proc;
pub mod syscall;

// The entry point for this OS
global_asm!(include_str!("boot/entry.S"));

pub fn init() {
    logger::init(LevelFilter::Debug).expect("logger init failed.");
    info!("Initializing the system...🤨");

    unsafe { mem::init() };
    proc::init();
    // interrupt::init();
}

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    use crate::syscall::shutdown;

    init();
    test_main();

    info!("It did not crash!");
    shutdown()
}

#[test_case]
fn test_assertion() {
    assert_eq!(1, 1);
}
