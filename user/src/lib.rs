#![no_std]
#![feature(linkage)]

use core::panic::PanicInfo;
use buddy_system_allocator::LockedHeap;

pub mod console;

extern crate alloc;

const KERNEL_HEAP_SIZE: usize = 1 * 1024 * 1024;

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

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

#[allow(static_mut_refs)]
pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}
