#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(yeli_os::test_runner)]
// The custom test frameworks feature generates a main function that
// calls test_runner, but this function is ignored because we use
// the #[no_main] attribute and provide our own entry point.
#![reexport_test_harness_main = "test_main"]

use log::info;
use yeli_os::{init, syscall::shutdown};

#[no_mangle]
pub extern "C" fn start() -> ! {
    init();
    info!("Welcome to YeLi-OS ~");

    #[cfg(test)]
    test_main();

    info!("It did not crash!");
    shutdown()
}
