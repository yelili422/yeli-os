#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
// The custom test frameworks feature generates a main function that
// calls test_runner, but this function is ignored because we use
// the #[no_main] attribute and provide our own entry point.
#![reexport_test_harness_main = "test_main"]
#![feature(global_asm)]
#![feature(asm)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate bitflags;
extern crate alloc;

global_asm!(include_str!("boot/entry.asm"));

pub mod config;
pub mod interrupt;
mod lang_items;
pub mod mm;
pub mod process;
pub mod syscall;
pub mod utils;

pub use lang_items::test_runner;
use log::info;
use utils::logger;

pub fn init() {
    logger::init().expect("the logger init failed.");
    info!("Initializing the system...");

    interrupt::init();
    mm::init();
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
