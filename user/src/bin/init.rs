#![no_std]
#![no_main]

use syscall::{exec, exit};

extern crate system;

#[no_mangle]
fn main() {
    exec("/bin/hello\0", &[]);
    exit(0);
}
