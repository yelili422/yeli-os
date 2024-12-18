#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(yeli_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use yeli_os::{init, syscall::shutdown};

#[no_mangle]
pub extern "C" fn _start(hart_id: usize, dtb_addr: usize) -> ! {
    init(hart_id, dtb_addr);
    test_main();
    shutdown()
}

#[test_case]
fn test_boot() {}
