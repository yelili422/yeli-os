use log::debug;

// mod allocator;
mod heap;
// mod page;
// mod segment;

pub const MEMORY_END: usize = 0x8080_0000;

pub unsafe fn init() {
    extern "C" {
        fn __text_start();
        fn __text_end();
        fn __rodata_start();
        fn __rodata_end();
        fn __data_start();
        fn __data_end();
        fn __bss_start();
        fn __bss_end();
    }

    debug!("Printing the default memory layout...");
    debug!(".text\t[{:#x}, {:#x})", __text_start as usize, __text_end as usize);
    debug!(".rodata\t[{:#x}, {:#x})", __rodata_start as usize, __rodata_end as usize);
    debug!(".data\t[{:#x}, {:#x})", __data_start as usize, __data_end as usize);
    debug!(".bss\t[{:#x}, {:#x})", __bss_start as usize, __bss_end as usize);

    heap::init();
    // allocator::init();
    // segment::init();
}
