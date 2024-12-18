use address::VirtualAddress;
use allocator::init_allocator;
use log::info;
use page::kvm_make;

use self::{
    address::{Address, MAX_VA},
    page::{enable_paging, PageSize, PageTable, Size4KiB},
};
use crate::lp2addr;

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

/// MMIO base address.
pub const VIRTIO_MMIO_BASE: Address = 0x1000_1000;

/// MMIO length.
pub const VIRTIO_MMIO_LEN: usize = 0x1000;

/// riscv default PLIC(Platform-Level Interrupt Controller) base address.
pub const PLIC_BASE: usize = 0x0C00_0000;

/// UART0 base address.
pub const UART0: usize = 0x1000_0000;

/// The address of trampoline.
pub const TRAMPOLINE: VirtualAddress = MAX_VA - PAGE_SIZE;

/// The address of trap frame.
pub const TRAP_FRAME: VirtualAddress = TRAMPOLINE - PAGE_SIZE;

/// User stack length
pub const USER_STACK_SIZE: usize = PAGE_SIZE * 4;

/// The address of user stack.
pub const USER_STACK: VirtualAddress = TRAP_FRAME - USER_STACK_SIZE - 1;

/// Kernel stack length
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 64; // FIXME: it's toooo big

/// Converts a linker identifier to address.
#[macro_export]
macro_rules! lp2addr {
    ($link_point:ident) => {
        #[allow(unused_unsafe)]
        unsafe {
            &($link_point) as *const _ as usize
        }
    };
}

extern "C" {
    /// The linker identifier of kernel end.
    static end: u8;

    /// The linker identifier of text end.
    static etext: u8;
}

pub fn init() {
    assert_eq!(size_of::<PageTable>(), PAGE_SIZE);

    info!("Initializing memory...");
    unsafe {
        init_allocator(lp2addr!(end), MEM_END);
        let kernel_pagetable = kvm_make();
        enable_paging(kernel_pagetable);
    }
}
