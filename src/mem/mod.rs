use crate::{addr, mem::allocator::alloc_one_page, proc::ContextId};

use self::{
    address::{as_mut, Address, VirtualAddress, MAX_VA},
    allocator::FRAME_ALLOCATOR,
    page::{enable_paging, PTEFlags, PageSize, PageTable, Size4KiB},
};

pub mod address;
pub mod allocator;
pub mod page;

/// The page size of kernel.
pub const PAGE_SIZE: u64 = Size4KiB::SIZE;

/// The start address of kernel.
// NOTE: Always keep same with `BASE_ADDRESS` in linker.ld.
pub const KERNEL_BASE: Address = 0x8020_0000;

/// The end address of physical memory.
pub const MEM_END: Address = KERNEL_BASE + 1024 * 1024 * 10;

/// The address of trampoline.
pub const TRAMPOLINE: Address = MAX_VA - PAGE_SIZE + 1;

/// The address of trap frame.
pub const TRAP_FRAME: Address = TRAMPOLINE - PAGE_SIZE;

/// The kernel stack address of this process.
pub const fn kernel_stack(pid: ContextId) -> VirtualAddress {
    TRAMPOLINE - (pid as u64 + 1) * 2 * PAGE_SIZE
}

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
unsafe fn kvm_make() -> &'static mut PageTable {
    let pa = alloc_one_page().expect("kvm_make: allocate page failed.");
    let pt = as_mut::<PageTable>(pa);

    // map kernel text executable and read-only.
    pt.map(KERNEL_BASE, KERNEL_BASE, addr!(etext) - KERNEL_BASE, PTEFlags::R | PTEFlags::X);

    // map kernel data and the physical RAM we'll make use of.
    pt.map(addr!(etext), addr!(etext), MEM_END - addr!(etext), PTEFlags::R | PTEFlags::W);

    // Map the trampoline for trap entry/exit to the hightest virtual
    // address in the kernel.
    // pt.map(TRAMPOLINE, addr!(trampoline), PAGE_SIZE, PTEFlags::R | PTEFlags::W);

    // Allocate a page for each process's kernel stack.
    // Map it high in memory, followed by an invalid
    // guard page.
    // for pid in 0..MAX_PROC {
    //     let page = alloc_one_page().expect("kvm_make: allocate kernel stack failed.");
    //     pt.map(kernel_stack(pid), page, PAGE_SIZE, PTEFlags::R | PTEFlags::W);
    // }

    pt
}

pub unsafe fn init() {
    FRAME_ALLOCATOR.init(addr!(end), MEM_END);

    let kernel_pagetable = kvm_make();
    enable_paging(kernel_pagetable.make_satp());
}
