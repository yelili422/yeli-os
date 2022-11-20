#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(yeli_os::lang_items::test_runner)]
#![reexport_test_harness_main = "test_main"]

use yeli_os::{init, syscall::shutdown};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init();
    test_main();
    shutdown()
}

#[test_case]
fn test_boot() {

}
