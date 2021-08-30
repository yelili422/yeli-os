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


mod lang_items;
mod init;
mod syscall;
mod console;
// mod vga_buffer;


pub fn test_runner(tests: &[&dyn Fn()]) {
    // println!("Running {} tests", tests.len());
    for test in tests {
        test();
        // exit_qemu(QemuExitCode::Failed);
    }
    // exit_qemu(QemuExitCode::Success);
}
