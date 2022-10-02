use log::debug;

mod alloc;
mod heap;
mod page;
mod segment;

use crate::mem::{
    page::PhysicalAddress,
    segment::{MapType, Permissions, Segment},
};
use ::alloc::vec;

pub const MEMORY_END: usize = 0x8080_0000;

macro_rules! addr {
    ($link_point:ident) => {
        &($link_point) as *const _ as usize
    };
}

extern "C" {
    static __kernel_start: u8;
    static __text_start: u8;
    static __text_end: u8;
    static __rodata_start: u8;
    static __rodata_end: u8;
    static __data_start: u8;
    static __data_end: u8;
    static __bss_start: u8;
    static __bss_end: u8;
    static __kernel_end: u8;
}

pub unsafe fn init() {
    debug!("Printing the default memory layout...");
    debug!(
        ".text\t[{:#x}, {:#x})",
        addr!(__text_start),
        addr!(__text_end)
    );
    debug!(
        ".rodata\t[{:#x}, {:#x})",
        addr!(__rodata_start),
        addr!(__rodata_end)
    );
    debug!(
        ".data\t[{:#x}, {:#x})",
        addr!(__data_start),
        addr!(__data_end)
    );
    debug!(".bss\t[{:#x}, {:#x})", addr!(__bss_start), addr!(__bss_end));

    heap::init();

    alloc::init(
        PhysicalAddress::from(addr!(__kernel_end)).ceil_page(),
        PhysicalAddress::from(MEMORY_END).floor_page(),
    );

    let segments = vec![
        Segment::new(
            addr!(__text_start).into(),
            addr!(__text_end).into(),
            MapType::Identical,
            Permissions::READABLE | Permissions::EXECUTABLE,
        ),
        Segment::new(
            addr!(__rodata_start).into(),
            addr!(__rodata_end).into(),
            MapType::Identical,
            Permissions::READABLE,
        ),
        Segment::new(
            addr!(__data_start).into(),
            addr!(__data_end).into(),
            MapType::Identical,
            Permissions::READABLE | Permissions::WRITABLE,
        ),
        Segment::new(
            addr!(__bss_start).into(),
            addr!(__bss_end).into(),
            MapType::Identical,
            Permissions::READABLE | Permissions::WRITABLE,
        ),
        Segment::new(
            addr!(__kernel_end).into(),
            (MEMORY_END as usize).into(),
            MapType::Identical,
            Permissions::READABLE | Permissions::WRITABLE,
        ),
    ];
    segment::init(segments);
}
