#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
// The custom test frameworks feature generates a main function that
// calls test_runner, but this function is ignored because we use
// the #[no_main] attribute and provide our own entry point.
#![reexport_test_harness_main = "test_main"]
#![feature(alloc_error_handler)]
#![feature(new_zeroed_alloc)]
#![feature(stmt_expr_attributes)]

extern crate alloc;

use core::{arch::global_asm, panic::PanicInfo};

use drivers::uart::uart_init;
use log::LevelFilter;
use proc::cpu::set_cpu_id;

pub use self::intr::shutdown;

mod console;
mod drivers;
mod fs;
mod intr;
mod logger;
mod mem;
mod proc;
mod sync;

// The entry point for this OS
global_asm!(include_str!("boot/entry.S"));

pub fn init(hart_id: usize, _dtb_addr: usize) {
    set_cpu_id(hart_id);

    // init uart for global output
    uart_init();

    logger::init(LevelFilter::Debug).unwrap();

    // enable kernel page table and virtual memory
    mem::init();

    fs::init();

    proc::init();

    intr::init();

    proc::schedule();
}

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(hart_id: usize, dtb_addr: usize) -> ! {
    use intr::shutdown;
    use log::info;

    init(hart_id, dtb_addr);
    test_main();

    info!("It did not crash!");
    shutdown()
}

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("[test] {} ...\t", core::any::type_name::<T>());
        self();
        print!("ok\n");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    println!("\n[test] Running {} test(s)...", tests.len());
    for test in tests {
        test.run();
    }
    println!("[test] Test finished.");
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use intr::shutdown;

    if let Some(location) = info.location() {
        println!("\n[panic] at {}:{} {}", location.file(), location.line(), info.message());
    } else {
        println!("[panic] {}", info.message());
    }
    shutdown()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use intr::shutdown;

    println!("\x1b[31m[test] failed\x1b[0m: {}\n", &info);
    shutdown()
}
