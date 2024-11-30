#![no_std]
#![feature(linkage)]

use core::panic::PanicInfo;

extern crate syscall;

pub mod console;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    main();
    panic!()
}

#[no_mangle]
#[linkage = "weak"]
fn main() -> i32 {
    unimplemented!()
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "\n[panic] at {}:{} {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("[panic] {}", info.message());
    }
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("failed\n{}\n", &info);
    loop {}
}
