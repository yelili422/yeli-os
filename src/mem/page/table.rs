use super::{frame::Frame, PhysicalPageNum, VirtualPageNum};
use crate::mem::allocator::frame_allocate;
use alloc::vec;
use alloc::vec::Vec;
use bit_field::BitField;
use core::fmt::{self, Debug, Formatter};

const FLAGS_RANGE: core::ops::Range<usize> = 0..8;

const PAGE_NUM_RANGE: core::ops::Range<usize> = 10..54;

bitflags! {
    #[derive(Default)]
    pub struct Flags: usize {
        const VALID = 1 << 0;
        const READABLE = 1 << 1;
        const WRITABLE = 1 << 2;
        const EXECUTABLE = 1 << 3;
        const USER = 1 << 4;
        const GLOBAL = 1 << 5;
        const ACCESSED = 1 << 6;
        const DIRTY = 1 << 7;
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(pub usize);

impl PageTableEntry {
    pub fn new(ppn: PhysicalPageNum, flags: Flags) -> Self {
        Self(
            *0usize
                .set_bits(FLAGS_RANGE, flags.bits)
                .set_bits(PAGE_NUM_RANGE, ppn.into()),
        )
    }

    pub fn empty() -> Self {
        Self(0)
    }

    pub fn physical_page_num(&self) -> PhysicalPageNum {
        PhysicalPageNum::from(self.0.get_bits(PAGE_NUM_RANGE))
    }

    pub fn flags(&self) -> Flags {
        unsafe { Flags::from_bits_unchecked(self.0.get_bits(FLAGS_RANGE)) }
    }

    pub fn is_valid(&self) -> bool {
        (self.flags() & Flags::VALID) != Flags::empty()
    }

    pub fn is_readable(&self) -> bool {
        (self.flags() & Flags::READABLE) != Flags::empty()
    }

    pub fn is_writable(&self) -> bool {
        (self.flags() & Flags::WRITABLE) != Flags::empty()
    }

    pub fn is_executable(&self) -> bool {
        (self.flags() & Flags::EXECUTABLE) != Flags::empty()
    }
}

impl Debug for PageTableEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageTableEntry")
            .field("value", &self.0)
            .field("physical_page_number", &self.physical_page_num())
            .field("flags", &self.flags())
            .finish()
    }
}

pub struct PageTable {
    root: PhysicalPageNum,
    frames: Vec<Frame>,
}

impl PageTable {
    pub fn new() -> Self {
        let frame = frame_allocate().unwrap();
        Self {
            root: frame.ppn(),
            frames: vec![frame],
        }
    }

    /// Find the page table entry corresponding the virtual page number.
    /// If not found, attempt to create new page table entry.
    ///
    /// Returns the mutable pointer of the target page table entry
    /// for subsequent operations of read and write.
    pub fn find_or_create(&mut self, vpn: VirtualPageNum) -> Option<&mut PageTableEntry> {
        let levels = vpn.levels();
        let mut p = self.root;
        let mut res: Option<&mut PageTableEntry> = None;
        for i in 0..3 {
            let entries = p.get_page_directory();
            let target_item = &mut entries[levels[i]];
            if !target_item.is_valid() && i < 2 {
                let frame = frame_allocate().unwrap();
                *target_item = PageTableEntry::new(frame.ppn(), Flags::VALID);
                self.frames.push(frame);
            }
            p = target_item.physical_page_num();
            res = Some(target_item);
        }
        res
    }

    pub fn find(&self, vpn: VirtualPageNum) -> Option<&PageTableEntry> {
        let levels = vpn.levels();
        let mut p = self.root;
        let mut res: Option<&PageTableEntry> = None;
        for i in 0..3 {
            let entries = p.get_page_directory();
            let target_item = &entries[levels[i]];
            if !target_item.is_valid() {
                return None;
            }
            p = target_item.physical_page_num();
            res = Some(target_item);
        }
        res
    }

    pub fn map(&mut self, vpn: VirtualPageNum, ppn: PhysicalPageNum, flags: Flags) {
        let pte = self.find_or_create(vpn).unwrap();
        assert!(!pte.is_valid(), "{:?} is mapped before mapping.", vpn);
        *pte = PageTableEntry::new(ppn, flags | Flags::VALID);
    }

    pub fn unmap(&mut self, vpn: VirtualPageNum) {
        let pte = self.find_or_create(vpn).unwrap();
        assert!(pte.is_valid(), "{:?} is invalid before unmapping.", vpn);
        *pte = PageTableEntry::empty();
    }

    pub fn token(&self) -> usize {
        8usize << 60 | self.root.0
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use crate::mem::{allocator::frame_allocate, page::{Frame, VirtualPageNum, table::{Flags, PageTable}}};

    #[test_case]
    fn test_write_to_page() {
        let frame = frame_allocate().unwrap();
        let bytes = frame.ppn().get_bytes_array();
        for (i, item) in bytes.iter_mut().enumerate() {
            *item = (i % 255) as u8;
        }
        for (i, item) in bytes.iter().enumerate() {
            assert_eq!(*item, (i % 255) as u8);
        }
    }

    #[test_case]
    fn test_alloc_pages() {
        let mut v: Vec<Frame> = Vec::new();
        for _ in 0..400 {
            let f = frame_allocate().unwrap();
            for iter in f.ppn().get_bytes_array().iter_mut() {
                *iter = 0;
            }
            v.push(f);
        }
    }

    #[test_case]
    fn test_find_page_by_vpn() {
        let frame = frame_allocate().unwrap();
        let mut pt = PageTable::new();
        let vpn = VirtualPageNum::from(0x0_0000);
        pt.map(vpn, frame.ppn(), Flags::READABLE);
        assert!(pt.find(vpn).is_some());
        assert!(!pt.find(VirtualPageNum::from(0x0_0001)).is_some())
    }
}
