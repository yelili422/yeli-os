use core::{
    arch::asm,
    fmt,
    ops::{Index, IndexMut},
};

use bit_field::BitField;
use bitflags::bitflags;
use log::{debug, trace};
use riscv::register::satp;

use crate::{
    mem::{
        address::{as_mut, px, PhysicalAddress, VirtualAddress, MAX_VA, PG_SHIFT},
        allocator::alloc_pages,
        PAGE_SIZE,
    },
    pa2va, pg_round_down,
};

// TODO: These methods only used for kernel address space.
/// Converts the virtual address to physical address.
#[macro_export]
macro_rules! va2pa {
    ($va:expr) => {
        // do nothing because of identical map in kernel.
        $va
    };
}

/// Converts the physical address to virtual address.
#[macro_export]
macro_rules! pa2va {
    ($pa:expr) => {
        $pa
    };
}

bitflags! {
    #[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
    pub struct PTEFlags: usize {
        const V = 1 << 0; // VALID
        const R = 1 << 1; // READABLE
        const W = 1 << 2; // WRITABLE
        const X = 1 << 3; // EXECUTABLE
        const U = 1 << 4; // USER
        const G = 1 << 5; // GLOBAL
        const A = 1 << 6; // ACCESSED
        const D = 1 << 7; // DIRTY
    }
}

/// Page table entry in risc-V Sv39 mod.
///
/// [54..63] - reserved.
/// [28..53] - 9 bits of level-2 index.
/// [19..27] - 9 bits of level-1 index.
/// [10..18] - 9 bits of level-0 index.
/// [8..9] - RSW, reserved for supervisor software.
/// [0..7] - flags, also see [`PTEFlags`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PTE(usize);

impl PTE {
    pub const fn empty() -> Self {
        PTE(0)
    }

    pub fn new(pa: PhysicalAddress, flags: PTEFlags) -> Self {
        let p = pa >> PG_SHIFT << 10;
        PTE(p | flags.bits())
    }

    pub fn pa(&self) -> PhysicalAddress {
        self.0 >> 10 << PG_SHIFT
    }

    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits_retain(self.0.get_bits(0..8))
    }

    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }

    pub fn is_directory(&self) -> bool {
        self.is_valid() && self.is_readable() && self.is_writable() && self.is_executable()
    }

    pub fn is_page(&self) -> bool {
        self.is_valid() && !self.is_directory()
    }

    pub fn is_readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    pub fn is_writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    pub fn is_executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

impl fmt::Display for PTE {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PTE(pa: 0x{:x}, flags: {:08b})", self.pa(), self.0.get_bits(0..7))
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct PageTable([PTE; 512]);

impl PageTable {
    pub const fn empty() -> Self {
        PageTable([PTE::empty(); 512])
    }

    pub fn iter(&self) -> impl Iterator<Item = &PTE> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PTE> {
        self.0.iter_mut()
    }

    pub unsafe fn map(
        &mut self,
        va: VirtualAddress,
        pa: PhysicalAddress,
        size: usize,
        perm: PTEFlags,
    ) {
        assert!(size > 0);
        debug!(
            "page_table: map 0x{:x} to 0x{:x}, size: {} bytes, flags: {:?}",
            va, pa, size, perm
        );

        let mut va = pg_round_down!(va, PAGE_SIZE);
        let mut pa = pg_round_down!(pa, PAGE_SIZE);
        let last = pg_round_down!(va + size - 1, PAGE_SIZE);

        loop {
            let pte = self.walk(va);
            if pte.is_valid() {
                panic!("remap at 0x{:x}, pte: {}.", va, pte);
            }

            *pte = PTE::new(pa, PTEFlags::V | perm);

            if va >= last {
                break;
            }

            va += PAGE_SIZE;
            pa += PAGE_SIZE;
        }
    }

    pub fn walk(&mut self, va: VirtualAddress) -> &mut PTE {
        assert!(va < MAX_VA);

        let mut page_table = self;
        for level in (1..3usize).rev() {
            let pte = page_table[px(level, va)];
            trace!("page_table_walk: check pte: {}, level: {}", pte, level);

            if pte.is_valid() {
                page_table = unsafe { as_mut(pte.pa()) };
                trace!("page_table_walk: valid");
            } else {
                let pa = alloc_pages(1).expect("paging alloc error");
                page_table[px(level, va)] = PTE::new(pa, PTEFlags::V);
                trace!("page_table_walk: invalid, create one: {}", page_table[px(level, va)]);
                page_table = unsafe { as_mut(pa2va!(pa)) };
            }
        }

        &mut page_table[px(0, va)]
    }

    /// Makes `satp` csr for enable paging.
    ///
    /// [60..63] - mode: values Bare, Sv39, and Sv48. use Sv39 here.
    /// [44..59] - address-space identifier.
    /// [ 0..43] - the physical page number of root page table.
    pub fn make_satp(&self) -> usize {
        let addr = self as *const _ as usize;
        8 << 60 | addr >> 12
    }
}

impl fmt::Display for PageTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PageTable(")?;

        for pte in self.0.iter() {
            if pte.is_valid() {
                write!(f, "{}, ", pte)?;
            }
        }

        write!(f, ")")?;
        Ok(())
    }
}

impl Index<usize> for PageTable {
    type Output = PTE;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

pub unsafe fn enable_paging(token: usize) {
    debug!("page_table: enable paging with satp: 0x{:x}", token);
    satp::write(token);
    asm!("sfence.vma"); // clear tlb
}

pub fn current_page_table() -> usize {
    satp::read().bits()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_map() {
        let mut pt = PageTable::empty();
        for pte in pt.iter() {
            assert_eq!(pte, &PTE::empty());
        }

        let va = 0x8000_0000;
        let pa = 0x1000_0000;

        unsafe {
            pt.map(va, pa, 0x1000, PTEFlags::R | PTEFlags::W);
        }

        let pte = pt.walk(va);
        assert!(pte.is_valid());
        assert!(pte.is_page());
        assert!(pte.is_readable());
        assert!(pte.is_writable());
        assert_eq!(pte.pa(), pg_round_down!(pa, PAGE_SIZE));
    }
}
