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

use alloc::sync::Arc;
use core::{arch::global_asm, panic::PanicInfo};

use console::HexDump;
use drivers::virtio::virtio_blk::VirtIOBlock;
use fs::FileSystem;
use log::{info, LevelFilter};
use mem::VIRTIO_MMIO_BASE;
use sync::once_cell::OnceCell;
use syscall;

pub mod console;
mod drivers;
pub mod intr;
pub mod logger;
pub mod mem;
pub mod proc;
mod sync;

// The entry point for this OS
global_asm!(include_str!("boot/entry.S"));

pub fn init(hart_id: usize, _dtb_addr: usize) {
    logger::init(LevelFilter::Debug).expect("logger init failed.");
    info!("Running on hart {}.", hart_id);
    info!("Initializing the system...");

    // match unsafe { dtb::Reader::read_from_address(dtb_addr) } {
    //     Ok(reader) => {
    //         let root = reader.struct_items();
    //         let (prop, _) = root.path_struct_items("/soc/plic").next().unwrap();
    //         println!("property: {:?}, {:?}", prop.name(), prop.unit_address());
    //     }
    //     Err(err) => {
    //         panic!("{:?}", err)
    //     }
    // }

    unsafe { mem::init() };
    init_fs();
    proc::init();
    intr::init();

    // info!("Start scheduling...");
    // proc::schedule();
}

fn init_fs() {
    match VirtIOBlock::init(VIRTIO_MMIO_BASE) {
        Ok(dev) => {
            let fs = FileSystem::open(dev, true).expect("failed to open file system");

            let bin_file = fs
                .get_inode_from_path("/bin/hello", &fs.root())
                .expect("failed to open file");
            let bin_file_guard = bin_file.lock();
            {
                let mut buf = [0u8; 4096];
                let mut offset = 0;
                loop {
                    let size = fs.read_inode(&bin_file_guard, offset, &mut buf);
                    println!("{}", HexDump(&buf[0..size]));

                    if size != buf.len() {
                        break;
                    }

                    offset += size;
                }
            }

            _ = ROOT_FS.set(fs);
        }
        Err(err) => panic!("{:?}", err),
    }
}

static ROOT_FS: OnceCell<Arc<FileSystem>> = OnceCell::new();

#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(hart_id: usize, dtb_addr: usize) -> ! {
    use crate::syscall::shutdown;

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
