#![no_std]
#![cfg_attr(not(test), no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![feature(panic_info_message)]
// #![feature(alloc_error_handler)]
#![feature(step_trait)]

// #[macro_use]
extern crate bitflags;
// extern crate alloc;

use core::arch::global_asm;

pub use lang_items::test_runner;
use log::{info, LevelFilter};

pub mod console;
pub mod interrupt;
mod lang_items;
pub mod logger;
pub mod mem;
pub mod proc;
pub mod syscall;

// The entry point for this OS
global_asm!(include_str!("boot/entry.S"));

pub fn init() {
    logger::init(LevelFilter::Trace).unwrap();
    info!("Initializing the system...");

    mem::init(&mem::MEMORY_MAP);
    // proc::init();
    // interrupt::init();

}
