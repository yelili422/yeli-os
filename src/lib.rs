#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
// The custom test frameworks feature generates a main function that
// calls test_runner, but this function is ignored because we use
// the #[no_main] attribute and provide our own entry point.
#![reexport_test_harness_main = "test_main"]
#![feature(global_asm)]
// FIXME:
// use of deprecated macro `llvm_asm`: will be removed from the compiler,
// use asm! instead
#![feature(llvm_asm)]
#![feature(panic_info_message)]

use log::info;

global_asm!(include_str!("boot/entry.asm"));

pub mod console;
pub mod interrupt;
pub mod logger;
mod lang_items;
pub mod syscall;

pub use lang_items::test_runner;

pub fn init() {
    logger::init().expect("the logger init failed.");
    info!("Initializing the system...");

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
