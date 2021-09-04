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
use crate::syscall::sbi::shutdown;

global_asm!(include_str!("boot/entry.asm"));


mod panic;
mod syscall;
mod interrupt;
mod console;
mod logger;


#[no_mangle]
pub fn rust_main() -> ! {
    interrupt::init();

    logger::init().expect("the logger init failed.");

    info!("Welcome to YeLi-OS ~");

    #[cfg(test)]
    test_main();

    shutdown()
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    info!("[test] Running {} test(s)...", tests.len());
    for test in tests {
        test();
    }
}


#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 2);
}
