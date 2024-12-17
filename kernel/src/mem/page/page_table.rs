use core::{
    arch::asm,
    fmt,
    ops::{Index, IndexMut},
    ptr::copy_nonoverlapping,
};

use bit_field::BitField;
use bitflags::bitflags;
use log::{debug, info, trace};
use riscv::register::satp;

use crate::{
    mem::{
        address::{as_mut, px, PhysicalAddress, VirtualAddress, MAX_VA, PG_SHIFT},
        allocator::FromRawPage,
        PAGE_SIZE,
    },
    pa2va, pg_round_down, pg_round_up, println,
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
#[repr(C, align(4))]
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

    pub fn is_empty(&self) -> bool {
        self.0 == 0
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
        if self.is_empty() {
            write!(f, "<empty> ")
        } else {
            write!(f, "0x{:x},{:?}", self.pa(), self.flags())
        }
    }
}

#[repr(C, align(4096))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageTable([PTE; PAGE_SIZE / size_of::<usize>()]);

impl PageTable {
    pub const fn empty() -> Self {
        PageTable([PTE::empty(); PAGE_SIZE / size_of::<usize>()])
    }

    pub fn user_vm_init(&mut self, src: &[u8]) {
        assert!(src.len() <= PAGE_SIZE, "user init data too large");

        let page = unsafe { PageTable::new_zeroed() };
        unsafe { copy_nonoverlapping(src.as_ptr(), page as *mut u8, PAGE_SIZE) };

        unsafe {
            self.map(
                VirtualAddress::from(0usize),
                PhysicalAddress::from(page as *mut u8 as usize),
                PAGE_SIZE,
                PTEFlags::R | PTEFlags::W | PTEFlags::X | PTEFlags::U,
            )
        };
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
            "page_table: map 0x{:x}-0x{:x} to 0x{:x}-0x{:x}, size: {} bytes, flags: {:?}",
            va,
            va + size,
            pa,
            pa + size,
            size,
            perm
        );

        let mut va = pg_round_down!(va, PAGE_SIZE);
        let mut pa = pg_round_down!(pa, PAGE_SIZE);
        let end = pg_round_up!(va + size, PAGE_SIZE);

        while va != end {
            trace!("page_table_map: mapping 0x{:x}", va);
            let pte = self.walk(va, true).expect("page_table_map: walk failed");
            if pte.is_valid() {
                panic!("remap at 0x{:x}, existing pte: {}.", va, pte);
            }

            *pte = PTE::new(pa, PTEFlags::V | perm);

            va += PAGE_SIZE;
            pa += PAGE_SIZE;
        }
    }

    pub fn walk(&mut self, va: VirtualAddress, alloc: bool) -> Option<&mut PTE> {
        assert!(va < MAX_VA, "virtual address out of range: 0x{:x}", va);

        let mut page_table = self;
        for level in (1..3usize).rev() {
            let pte: PTE = page_table[px(level, va)];

            if pte.is_valid() {
                page_table = unsafe { as_mut(pa2va!(pte.pa())) };
                trace!("page_table_walk: check pte: {}, level: {}, valid", pte, level);
            } else {
                assert_eq!(
                    pte,
                    PTE::empty(),
                    "Invalid pte also should be empty because the page table \
                    has been initialized with zero. Current page table: {}",
                    page_table
                );

                if !alloc {
                    return None;
                }
                let pa = unsafe { PageTable::new_zeroed() };
                page_table[px(level, va)] = PTE::new(pa, PTEFlags::V);
                trace!(
                    "page_table_walk: check pte: {}, level: {}, invalid. create one",
                    pte,
                    level
                );
                page_table = unsafe { as_mut(pa2va!(pa)) };
            }
        }

        Some(&mut page_table[px(0, va)])
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
        let base_address = self as *const _ as usize;
        let ptes = &self.0;
        write!(f, "PageTable at {:#x}\n", base_address)?;

        // Print each row with 4 PTEs per line
        for (i, chunk) in ptes.chunks(4).enumerate() {
            // Calculate the base address for the current row
            let row_address = base_address + i * 4 * size_of::<PTE>();

            // Write the address in hexadecimal format
            write!(f, "{:#18x}: ", row_address)?;

            // Print each PTE in the current row
            for pte in chunk {
                write!(f, "{} ", pte)?;
            }
            writeln!(f)?; // Newline at the end of each row
        }

        Ok(())
    }
}

impl FromRawPage for PageTable {}

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

pub unsafe fn enable_paging(pagetable: &PageTable) {
    let token = pagetable.make_satp();
    info!("page_table: enable paging with satp: 0x{:x}, {}", token, pagetable);
    satp::write(token);
    asm!("sfence.vma"); // clear tlb
}

pub fn current_page_table() -> usize {
    satp::read().bits()
}

#[repr(C, align(4096))]
pub struct RawPage([u8; PAGE_SIZE]);

impl FromRawPage for RawPage {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_walk() {
        let mut pt = PageTable::empty();

        for pte in pt.iter() {
            assert_eq!(pte, &PTE::empty());
        }

        let va = 0x8000_0000;
        let pa = 0x1000_0000;

        let pte = pt.walk(va, false);
        assert!(pte.is_none());

        unsafe {
            pt.map(va, pa, PAGE_SIZE, PTEFlags::R | PTEFlags::W);
        }

        let pte = pt.walk(va, false).unwrap();
        assert_eq!(pte, &PTE::new(pa, PTEFlags::R | PTEFlags::W | PTEFlags::V));

        let pte = pt.walk(va, true).unwrap();
        assert_eq!(pte, &PTE::new(pa, PTEFlags::R | PTEFlags::W | PTEFlags::V));

        let pte = pt.walk(va, false);
        assert!(pte.is_some());
    }

    #[test_case]
    fn test_continuous_mapping() {
        let mut pt = PageTable::empty();
        for pte in pt.iter() {
            assert_eq!(pte, &PTE::empty());
        }

        let va = 0x8000_0000;
        let pa = 0x1000_0000;

        unsafe {
            pt.map(va, pa, 0x1000, PTEFlags::R | PTEFlags::W);
            pt.map(va + 0x1000, pa, PAGE_SIZE, PTEFlags::R | PTEFlags::W);
        }

        let pte = pt.walk(va, true).unwrap();
        assert!(pte.is_valid());
        assert!(pte.is_page());
        assert!(pte.is_readable());
        assert!(pte.is_writable());
        assert_eq!(pte.pa(), pg_round_down!(pa, PAGE_SIZE));
    }

    // #[test_case]
    // fn test_map_capacity() {
    //     let mut pt = PageTable::empty();
    //     for va in (0..MAX_VA).step_by(PAGE_SIZE) {
    //         unsafe {
    //             pt.map(va, 0x1000_0000, PAGE_SIZE, PTEFlags::R | PTEFlags::W);
    //             assert_eq!(
    //                 pt.walk(va, false).unwrap(),
    //                 &PTE::new(0x1000_0000, PTEFlags::R | PTEFlags::W | PTEFlags::V)
    //             );
    //         }
    //     }
    //     assert_eq!(1, 1);
    // }
}
