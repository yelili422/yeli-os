#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
// The custom test frameworks feature generates a main function that
// calls test_runner, but this function is ignored because we use
// the #[no_main] attribute and provide our own entry point.
#![reexport_test_harness_main = "test_main"]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate bitflags;
extern crate alloc;

pub mod interrupt;
mod lang_items;
pub mod mem;
pub mod task;
pub mod syscall;
pub mod utils;

use core::arch::global_asm;

pub use lang_items::test_runner;
use log::info;
use utils::logger;

// The entry point for this OS
global_asm!(include_str!("boot/entry.asm"));

pub fn init() {
    logger::init().expect("The logger init failed.");
    info!("Initializing the system...");

    unsafe {mem::init();}
    interrupt::init();
}

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    use syscall::shutdown;

    init();
    test_main();
    shutdown()
}

#[test_case]
fn test_assertion() {
    assert_eq!(1, 1);
}
