/// A 64-bit physical address is split into three fields:
///
/// [56..63] - must be zero.
/// [12..55] - the physical page number.
/// [0..11] - 12 bits of byte offset within the page.
pub type PhysAddr = u64;

/// The risc-v Sv39 scheme has three levels of page-table
/// pages. A 64-bit virtual address is split into five fields:
///
/// [39..63] - must be zero.
/// [30..38] - 9 bits of level-2 index.
/// [21..29] - 9 bits of level-1 index.
/// [12..20] - 9 bits of level-0 index.
/// [ 0..11] - 12 bits of byte offset within the page.
pub type VirtAddr = u64;

/// MAX_VA is actually one bit less than the max allowed by
/// Sv39, to avoid having to sign-extend virtual addresses
/// that have the high bit set.
pub const MAX_VA: u64 = 1 << (9 + 9 + 9 + 12 - 1);

/// Bits of offset within a page.
pub const PG_SHIFT: u64 = 12;

#[derive(Debug)]
pub struct AddressNotAlignedError();

#[macro_export]
macro_rules! pg_round_up {
    ($sz:expr, $pg_size:expr) => {{
        ($sz + $pg_size - 1) & !($pg_size - 1)
    }};
}

#[macro_export]
macro_rules! pg_round_down {
    ($a:expr, $pg_size:expr) => {{
        $a & !($pg_size - 1)
    }};
}

#[macro_export]
macro_rules! is_aligned {
    ($addr:expr, $pg_size:expr) => {{
        crate::pg_round_down!($addr, $pg_size) == $addr
    }};
}

#[macro_export]
macro_rules! memset {
    ($addr:expr, $val:expr, $size:expr) => {
        use core::slice::from_raw_parts_mut;
        unsafe {
            for i in from_raw_parts_mut($addr as *mut u8, $size as usize).iter_mut() {
                *i = $val;
            }
        }
    };
}

/// Extract the three 9-bit page table indices from a virtual address.
pub fn px(level: usize, va: VirtAddr) -> usize {
    let level = level as u64;
    const PX_MUSK: u64 = 0x1FF; // 9 bits
    (va >> (PG_SHIFT + 9 * level) & PX_MUSK) as usize
}

/// Converts the physical address to T.
pub fn pa_as_mut<T>(pa: PhysAddr) -> &'static mut T {
    unsafe { (pa as *mut T).as_mut().unwrap() }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test_case]
    pub fn test_page_ground() {
        assert!(pg_round_up!(4095, 4096) == 4096);
        assert!(pg_round_up!(4096, 4096) == 4096);
        assert!(pg_round_up!(4097, 4096) == 8192);

        assert!(pg_round_up!(0, 1) == 0);
        assert!(pg_round_up!(1234, 1) == 1234);
        assert!(pg_round_up!(0xffff, 1) == 0xffff);

        assert!(pg_round_down!(4095, 4096) == 0);
        assert!(pg_round_down!(4096, 4096) == 4096);
        assert!(pg_round_down!(4097, 4096) == 4096);

        assert!(pg_round_down!(0, 1) == 0);
        assert!(pg_round_down!(1234, 1) == 1234);
        assert!(pg_round_down!(0xffff, 1) == 0xffff);
    }
}
