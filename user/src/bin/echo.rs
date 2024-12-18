#![no_std]
#![no_main]

use syscall::exit;
use system::{console::Stdin, print};

extern crate system;
extern crate alloc;

#[no_mangle]
fn main() {
    let mut buffer = [0; 1024];
    match Stdin.read(buffer.as_mut()) {
        Ok(size) => {
            let s = core::str::from_utf8(&buffer[..size]).unwrap_or(&"Invalid UTF-8");
            print!("{}", s);
        },
        Err(_) => {},
    }
    exit(0)
}
