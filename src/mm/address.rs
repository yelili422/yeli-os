use core::fmt::{self, Debug, Formatter};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(pub usize);

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(pub usize);

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalPageNum(pub usize);

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualPageNum(pub usize);

impl Debug for VirtualAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}

impl Debug for PhysicalAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}

impl Debug for PhysicalPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}

impl Debug for VirtualPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:#x}", self.0))
    }
}

impl From<usize> for PhysicalAddress {
    fn from(v: usize) -> Self {
        Self(v)
    }
}

impl From<usize> for VirtualAddress {
    fn from(v: usize) -> Self {
        Self(v)
    }
}

impl From<usize> for PhysicalPageNum {
    fn from(v: usize) -> Self {
        Self(v)
    }
}

impl From<usize> for VirtualPageNum {
    fn from(v: usize) -> Self {
        Self(v)
    }
}

impl From<PhysicalAddress> for usize {
    fn from(v: PhysicalAddress) -> Self {
        v.0
    }
}

impl From<VirtualAddress> for usize {
    fn from(v: VirtualAddress) -> Self {
        v.0
    }
}

impl From<PhysicalPageNum> for usize {
    fn from(v: PhysicalPageNum) -> Self {
        v.0
    }
}

impl From<VirtualPageNum> for usize {
    fn from(v: VirtualPageNum) -> Self {
        v.0
    }
}
