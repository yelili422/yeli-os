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

extern crate alloc;

use core::{arch::global_asm, panic::PanicInfo};

use log::{info, LevelFilter};
use syscall;

pub mod console;
pub mod intr;
pub mod logger;
pub mod mem;
pub mod proc;

// The entry point for this OS
global_asm!(include_str!("boot/entry.S"));

pub fn init() {
    logger::init(LevelFilter::Debug).expect("logger init failed.");
    info!("Initializing the system...");

    unsafe { mem::init() };
    proc::init();
    intr::init();

    // info!("Start scheduling...");
    proc::schedule();
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
    // TODO: parse args...

    // run tests
    println!("\n[test] Running {} test(s)...", tests.len());
    for test in tests {
        test.run();
    }
    println!("[test] Test finished.");

    // TODO: communicate through stdio

    // TODO: exit code
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!("\n[panic] at {}:{} {}", location.file(), location.line(), info.message());
    } else {
        println!("[panic] {}", info.message());
    }
    syscall::shutdown()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\x1b[31m[test] failed\x1b[0m: {}\n", &info);
    syscall::shutdown()
}
