pub trait PageSize: Copy + Eq + PartialOrd + Ord {
    const SIZE: usize;
    const SIZE_AS_DEBUG_STR: &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size4KiB {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size2MiB {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size1GiB {}

impl PageSize for Size4KiB {
    const SIZE: usize = 4096;
    const SIZE_AS_DEBUG_STR: &'static str = "4KiB";
}

impl PageSize for Size2MiB {
    const SIZE: usize = Size4KiB::SIZE * 512;
    const SIZE_AS_DEBUG_STR: &'static str = "2MiB";
}

impl PageSize for Size1GiB {
    const SIZE: usize = Size2MiB::SIZE * 512;
    const SIZE_AS_DEBUG_STR: &'static str = "1GiB";
}
