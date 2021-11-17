use super::{
    allocator::frame_allocate,
    page::{Flags, Frame, PageTable, PhysicalPageNum, VirtualAddress, VirtualPageNum},
};
use crate::{
    memory::{page::PAGE_SIZE, MEMORY_END},
    utils::range::ObjectRange,
};
use alloc::{collections::BTreeMap, sync::Arc, vec, vec::Vec};
use core::fmt::Debug;
use lazy_static::lazy_static;
use log::debug;
use riscv::register::satp;
use spin::Mutex;

pub fn init() {
    KERNEL_SPACE.lock().activate();
}

lazy_static! {
    pub static ref KERNEL_SPACE: Arc<Mutex<SegmentTable>> =
        Arc::new(Mutex::new(SegmentTable::new_kernel()));
}

bitflags! {
    #[derive(Default)]
    pub struct Permissions: usize {
        const READABLE = 1 << 1;
        const WRITABLE = 1 << 2;
        const EXECUTABLE = 1 << 3;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapType {
    /// The constant mapping can visit the specific physical address directly.
    Identical,
    /// Framed represents mapping a new allocated physical frame
    /// for each virtual page.
    Framed,
}

pub struct Segment {
    map_type: MapType,
    range: ObjectRange<VirtualPageNum>,
    /// Binding the frames' life cycle to the logic segment.
    frames: BTreeMap<VirtualPageNum, Frame>,
    permissions: Permissions,
}

impl Debug for Segment {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Segment")
            .field("map_type", &self.map_type)
            .field("range", &self.range)
            .field("frames", &self.frames)
            .field("permissions", &self.permissions)
            .finish()
    }
}

impl Segment {
    pub fn new(
        start: VirtualAddress,
        end: VirtualAddress,
        map_type: MapType,
        permissions: Permissions,
    ) -> Self {
        Self {
            map_type,
            permissions,
            frames: BTreeMap::new(),
            range: ObjectRange::new(start.floor_page(), end.ceil_page()),
        }
    }

    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.range {
            let ppn: PhysicalPageNum;
            match self.map_type {
                MapType::Identical => ppn = PhysicalPageNum(vpn.0),
                MapType::Framed => {
                    let frame = frame_allocate().unwrap();
                    ppn = frame.ppn();
                    self.frames.insert(vpn, frame);
                }
            }
            let flags = Flags::from_bits(self.permissions.bits).unwrap();
            page_table.map(vpn, ppn, flags);
        }
    }

    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.range {
            if let MapType::Framed = self.map_type {
                self.frames.remove(&vpn);
            }
            page_table.unmap(vpn);
        }
    }

    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let length = data.len();
        loop {
            let src = &data[start..length.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .find(self.range.get_start())
                .unwrap()
                .physical_page_num()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;

            if start >= length {
                break;
            }
        }
    }
}

pub struct SegmentTable {
    page_table: PageTable,
    segments: Vec<Segment>,
}

impl SegmentTable {
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            segments: Vec::new(),
        }
    }

    fn push(&mut self, mut segment: Segment, data: Option<&[u8]>) {
        segment.map(&mut self.page_table);
        if let Some(data) = data {
            segment.copy_data(&mut self.page_table, data);
        }
        self.segments.push(segment);
    }

    pub fn new_kernel() -> Self {
        debug!("Init the kernel's segments...");
        extern "C" {
            fn text_start();
            fn rodata_start();
            fn data_start();
            fn bss_start();
            fn kernel_end();
        }
        let mut res = Self::new_bare();
        let segments = vec![
            Segment::new(
                (text_start as usize).into(),
                (rodata_start as usize).into(),
                MapType::Identical,
                Permissions::READABLE | Permissions::EXECUTABLE,
            ),
            Segment::new(
                (rodata_start as usize).into(),
                (data_start as usize).into(),
                MapType::Identical,
                Permissions::READABLE,
            ),
            Segment::new(
                (data_start as usize).into(),
                (bss_start as usize).into(),
                MapType::Identical,
                Permissions::READABLE | Permissions::WRITABLE,
            ),
            Segment::new(
                (bss_start as usize).into(),
                (kernel_end as usize).into(),
                MapType::Identical,
                Permissions::READABLE | Permissions::WRITABLE,
            ),
            Segment::new(
                (kernel_end as usize).into(),
                (MEMORY_END as usize).into(),
                MapType::Identical,
                Permissions::READABLE | Permissions::WRITABLE,
            ),
        ];
        for segment in segments {
            debug!("Mapping: {:?}", &segment);
            res.push(segment, None);
        }
        res
    }

    pub fn activate(&self) {
        debug!("Activate the segment table...");
        let token = self.page_table.token();
        unsafe {
            satp::write(token);
            asm!("sfence.vma");
        }
    }
}
