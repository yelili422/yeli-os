use core::{
    arch::asm,
    fmt,
    ops::{Index, IndexMut},
};

use bit_field::BitField;
use bitflags::bitflags;
use log::trace;
use riscv::register::satp;

use crate::{
    mem::{
        address::{pa_as_mut, px, PhysAddr, VirtAddr, MAX_VA, PG_SHIFT},
        allocate, KERNEL_PG_SIZE,
    },
    memset, pg_round_down,
};

bitflags! {
    #[derive(Default)]
    pub struct PTEFlags: u64 {
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
#[derive(Clone, Copy, Debug)]
pub struct PTE(u64);

impl PTE {
    pub fn from_pa(pa: PhysAddr, flags: PTEFlags) -> Self {
        let p = pa >> PG_SHIFT << 10;
        PTE(p | flags.bits())
    }

    pub fn pa(&self) -> PhysAddr {
        self.0 >> 10 << PG_SHIFT
    }

    pub fn flags(&self) -> PTEFlags {
        unsafe { PTEFlags::from_bits_unchecked(self.0.get_bits(0..8)) }
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
        write!(
            f,
            "PTE(levels: [{}, {}, {}], flags: {:08b})",
            self.0.get_bits(28..53),
            self.0.get_bits(19..27),
            self.0.get_bits(10..18),
            self.0.get_bits(0..7)
        )
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct PageTable([PTE; 512]);

impl PageTable {
    pub fn iter(&self) -> impl Iterator<Item = &PTE> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PTE> {
        self.0.iter_mut()
    }

    pub fn map(&mut self, va: VirtAddr, pa: PhysAddr, size: u64, perm: PTEFlags) {
        assert!(size > 0);
        trace!(
            "page_table: map 0x{:x} to 0x{:x}, size: {}, perm: {:b}",
            va,
            pa,
            size,
            perm.bits()
        );

        let mut va = pg_round_down!(va, KERNEL_PG_SIZE);
        let mut pa = pa;
        let last = pg_round_down!(va + size - 1, KERNEL_PG_SIZE);

        loop {
            let pte = self.walk(va);
            if pte.is_valid() {
                panic!("remap at 0x{:x}, pte: {}.", va, pte);
            }

            *pte = PTE::from_pa(pa, PTEFlags::V | perm);

            if va >= last {
                break;
            }

            va += KERNEL_PG_SIZE;
            pa += KERNEL_PG_SIZE;
        }
    }

    pub fn walk(&mut self, va: VirtAddr) -> &mut PTE {
        assert!(va < MAX_VA);

        let mut page_table = self;
        for level in (1..3usize).rev() {
            let pte = page_table[px(level, va)];
            trace!("page_table_walk: check pte: {}, level: {}", pte, level);

            if pte.is_valid() {
                page_table = pa_as_mut(pte.pa());
                trace!("page_table_walk: valid");
            } else {
                let pa = allocate().unwrap();
                memset!(pa, 0, KERNEL_PG_SIZE);
                page_table[px(level, va)] = PTE::from_pa(pa, PTEFlags::V);
                trace!("page_table_walk: invalid, create one: {}", page_table[px(level, va)]);
                page_table = pa_as_mut(pa);
            }
        }

        &mut page_table[px(0, va)]
    }

    /// Makes `satp` csr for enable paging.
    ///
    /// [60..63] - mode: values Bare, Sv39, and Sv48. use Sv39 here.
    /// [44..59] - address-space identifier.
    /// [ 0..43] - the physical page number of root page table.
    pub fn make_satp(&self) -> u64 {
        let addr = self as *const _ as u64;
        8u64 << 60 | addr >> 12
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

pub unsafe fn enable_paging(token: u64) {
    satp::write(token as usize);
    asm!("sfence.vma"); // clear tlb
}
