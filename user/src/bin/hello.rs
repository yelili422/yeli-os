#![no_std]
#![no_main]

use syscall::exit;
use system::println;

extern crate system;

#[no_mangle]
fn main() {
    for i in 0..10 {
        println!("{}", i);
    }
    println!("Hello, world!");
    exit(0);
}
