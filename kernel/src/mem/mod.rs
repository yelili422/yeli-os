use allocator::{init_allocator, FromRawPage};
use log::info;

use self::{
    address::{as_mut, Address, VirtualAddress, MAX_VA},
    page::{enable_paging, PTEFlags, PageSize, PageTable, Size4KiB},
};
use crate::{intr::trampoline, lp2addr, proc::TaskId};

pub mod address;
pub mod allocator;
pub mod page;

/// The page size of kernel.
pub const PAGE_SIZE: usize = Size4KiB::SIZE;

/// The start address of kernel.
// NOTE: Always keep same with `BASE_ADDRESS` in linker.ld.
pub const KERNEL_BASE: Address = 0x8020_0000;

/// The end address of physical memory.
pub const MEM_END: Address = 0x8000_0000 + 1024 * 1024 * 128;

/// The address of trampoline.
pub const TRAMPOLINE: Address = MAX_VA - PAGE_SIZE;

/// The address of trap frame.
pub const TRAPFRAME: Address = TRAMPOLINE - PAGE_SIZE;

/// MMIO base address.
pub const VIRTIO_MMIO_BASE: Address = 0x1000_1000;

/// MMIO length.
pub const VIRTIO_MMIO_LEN: usize = 0x1000;

/// The kernel stack address of this process.
pub const fn kernel_stack(pid: TaskId) -> VirtualAddress {
    TRAMPOLINE - (pid as usize + 1) * 2 * PAGE_SIZE
}

/// Converts a linker identifier to address.
#[macro_export]
#[allow(unused_unsafe)]
macro_rules! lp2addr {
    ($link_point:ident) => {
        unsafe { &($link_point) as *const _ as usize }
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
    info!("page_table: initializing kernel page table...");

    let pt = unsafe {
        let page = PageTable::new_zeroed();
        info!("page_table: init page table at 0x{:x}", page);
        as_mut::<PageTable>(page)
    };

    // map kernel text executable and read-only.
    info!("page_table: mapping kernel text section...");
    pt.map(
        KERNEL_BASE,
        KERNEL_BASE,
        lp2addr!(etext) - KERNEL_BASE,
        PTEFlags::R | PTEFlags::X,
    );

    // map kernel data and the physical RAM we'll make use of.
    info!("page_table: mapping kernel data section...");
    pt.map(
        lp2addr!(etext),
        lp2addr!(etext),
        MEM_END - lp2addr!(etext),
        PTEFlags::R | PTEFlags::W,
    );

    // Map the trampoline for trap entry/exit to the hightest virtual
    // address in the kernel.
    info!("page_table: mapping trampoline...");
    pt.map(TRAMPOLINE, trampoline as usize, PAGE_SIZE, PTEFlags::R | PTEFlags::X);

    // Allocate a page for each process's kernel stack.
    // Map it high in memory, followed by an invalid
    // guard page.
    // for pid in 0..MAX_PROC {
    //     let page = alloc_one_page().expect("kvm_make: allocate kernel stack failed.");
    //     pt.map(kernel_stack(pid), page, PAGE_SIZE, PTEFlags::R | PTEFlags::W);
    // }

    info!("page_table: mapping MMIO section...");
    pt.map(VIRTIO_MMIO_BASE, VIRTIO_MMIO_BASE, VIRTIO_MMIO_LEN, PTEFlags::R | PTEFlags::W);

    pt
}

pub unsafe fn init() {
    assert_eq!(size_of::<PageTable>(), PAGE_SIZE);

    info!("Initializing memory...");
    init_allocator(lp2addr!(end), MEM_END);

    let kernel_pagetable = kvm_make();
    enable_paging(kernel_pagetable);
    info!("page_table: initialized.");
}
