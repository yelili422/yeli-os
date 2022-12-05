use crate::{addr, mem::allocator::allocate, memset};

use self::{
    address::pa_as_mut,
    allocator::FRAME_ALLOCATOR,
    page::{enable_paging, PTEFlags, PageSize, PageTable, Size4KiB},
};

pub mod address;
mod allocator;
pub mod page;

/// The start address of kernel.
const KERNEL_BASE: u64 = 0x8020_0000;

/// The end address of physical memory.
const MEM_END: u64 = KERNEL_BASE + 1024 * 1024 * 10;

/// The page size of kernel.
const KERNEL_PG_SIZE: u64 = Size4KiB::SIZE;

/// Converts a linker identifier to address.
#[macro_export]
#[allow(unused_unsafe)]
macro_rules! addr {
    ($link_point:ident) => {
        unsafe { &($link_point) as *const _ as u64 }
    };
}

extern "C" {
    /// The linker identifier of kernel end.
    static end: u8;

    /// The linker identifier of text end.
    static etext: u8;
}

/// Make a direct map page table for the kernel.
fn kvmmake() -> &'static mut PageTable {
    let pa = allocate().expect("alloc root page table failed.");
    memset!(pa, 0, KERNEL_PG_SIZE);

    let pt = pa_as_mut::<PageTable>(pa);

    // map kernel text executable and read-only.
    pt.map(KERNEL_BASE, KERNEL_BASE, addr!(etext) - KERNEL_BASE, PTEFlags::R | PTEFlags::X);

    // map kernel data and the physical RAM we'll make use of.
    pt.map(addr!(etext), addr!(etext), MEM_END - addr!(etext), PTEFlags::R | PTEFlags::W);

    pt
}

pub fn init() {
    #[allow(unused_unsafe)]
    unsafe {
        FRAME_ALLOCATOR.init(addr!(end), MEM_END);

        let kernel_pagetable = kvmmake();
        enable_paging(kernel_pagetable.make_satp());
    }
}
