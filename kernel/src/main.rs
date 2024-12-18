#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(yeli_os::test_runner)]

use log::info;
use yeli_os::{init, shutdown};

#[no_mangle]
pub extern "C" fn _start(hart_id: usize, dtb_addr: usize) -> ! {
    init(hart_id, dtb_addr);

    info!("Welcome to YeLi-OS ~");
    info!("It did not crash!");

    shutdown()
}
