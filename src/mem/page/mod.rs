mod frame;
mod table;

pub use {self::frame::*, self::table::*};

use crate::mem::page::table::PageTableEntry;
use crate::utils::range::StepByOne;
use bit_field::BitField;

pub const PAGE_SIZE: usize = 4096; // 4K

#[repr(C)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VirtualAddress(usize);

#[repr(C)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PhysicalAddress(usize);

#[repr(C)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VirtualPageNum(usize);

#[repr(C)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PhysicalPageNum(usize);

macro_rules! bitarray_type_impl {
    ($($t:ty)*) => ($(
        impl core::fmt::Debug for $t {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_fmt(format_args!("{:#x}", self.0))
            }
        }
        impl From<$t> for usize {
            fn from(v: $t) -> Self {
                v.0
            }
        }
        impl From<usize> for $t {
            fn from(v: usize) -> Self {
                Self(v)
            }
        }
        impl $t {
            pub fn value(&self) -> usize{
                self.0
            }
        }
    )*)
}

bitarray_type_impl! { VirtualAddress PhysicalAddress VirtualPageNum PhysicalPageNum }

macro_rules! address_type_impl {
    ($address: ty, $page_num: ty) => {
        impl From<$page_num> for $address {
            fn from(page_num: $page_num) -> Self {
                Self(page_num.0 * PAGE_SIZE)
            }
        }
        impl From<$address> for $page_num {
            fn from(address: $address) -> Self {
                assert!(
                    address.0 % PAGE_SIZE == 0,
                    "Converting unaligned address is not allowed."
                );
                Self(address.0 / PAGE_SIZE)
            }
        }
        impl $address {
            pub fn floor_page(&self) -> $page_num {
                <$page_num>::from(self.0 / PAGE_SIZE)
            }
            pub fn ceil_page(&self) -> $page_num {
                <$page_num>::from(self.0 / PAGE_SIZE + (self.0 % PAGE_SIZE != 0) as usize)
            }
            pub fn page_offset(&self) -> usize {
                self.0 % PAGE_SIZE
            }
        }
    };
}

address_type_impl!(PhysicalAddress, PhysicalPageNum);
address_type_impl!(VirtualAddress, VirtualPageNum);

impl VirtualPageNum {
    pub fn levels(self) -> [usize; 3] {
        [
            self.0.get_bits(18..27),
            self.0.get_bits(9..18),
            self.0.get_bits(0..9),
        ]
    }
}

impl PhysicalPageNum {
    pub fn get_page_directory(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysicalAddress = self.clone().into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, PAGE_SIZE / 8) }
    }

    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysicalAddress = self.clone().into();
        unsafe { core::slice::from_raw_parts_mut(pa.clone().0 as *mut u8, PAGE_SIZE) }
    }

    pub fn get_mut<T>(&self) -> &'static mut T {
        let pa: PhysicalAddress = self.clone().into();
        unsafe { (pa.0 as *mut T).as_mut().unwrap() }
    }
}

impl StepByOne for VirtualPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}
