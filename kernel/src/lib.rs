#![no_std]

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


mod lang_items;
mod syscall;
mod interrupt;
mod console;
mod logger;


pub fn test_runner(tests: &[&dyn Fn()]) {
    // println!("Running {} tests", tests.len());
    for test in tests {
        test();
        // exit_qemu(QemuExitCode::Failed);
    }
    // exit_qemu(QemuExitCode::Success);
}


fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) }
    });
}

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    interrupt::init();

    logger::init().expect("the logger init failed.");

    info!("Welcome to YeLi-OS ~");

    shutdown()
}
