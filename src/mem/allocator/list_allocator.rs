use core::{fmt, ptr::null_mut, slice};

use log::trace;

use crate::{
    is_aligned,
    mem::{address::PhysAddr, allocator::Allocator, KERNEL_PG_SIZE},
    pg_round_up,
};

#[repr(C)]
struct Link {
    next: *mut Link,
}

#[derive(Debug)]
pub struct ListAllocator {
    pa_start: PhysAddr,
    pa_end: PhysAddr,
    free_list: *mut Link,
}

impl ListAllocator {
    pub fn new(pa_start: PhysAddr, pa_end: PhysAddr) -> Self {
        ListAllocator {
            pa_start: pg_round_up!(pa_start, KERNEL_PG_SIZE),
            pa_end,
            free_list: null_mut(),
        }
    }

    pub fn free_range(&mut self) {
        let mut p = self.pa_start;
        while p <= self.pa_end {
            self.free(p);
            p += KERNEL_PG_SIZE;
        }
        trace!("allocator: free range from 0x{:x} to 0x{:x} finished.", self.pa_start, p);
    }
}

impl fmt::Display for ListAllocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ListAllocator(")?;
        write!(f, "from: 0x{:x}, to: 0x{:x}, free_list: ", self.pa_start, self.pa_end)?;

        let mut p = self.free_list;
        while p != null_mut() {
            write!(f, "0x{:x}, ", p as usize)?;
            unsafe {
                p = (*p).next;
            }
        }

        write!(f, ")")?;

        Ok(())
    }
}

impl Allocator for ListAllocator {
    fn free(&mut self, pa: PhysAddr) {
        assert!(is_aligned!(pa, KERNEL_PG_SIZE));
        assert!(pa >= self.pa_start && pa <= self.pa_end);

        unsafe {
            for p in slice::from_raw_parts_mut(pa as *mut u8, KERNEL_PG_SIZE as usize) {
                *p = 1; // Fill with junk to catch dangling refs.
            }

            let r = pa as *mut Link;

            (*r).next = self.free_list;
            self.free_list = r;
        }
    }

    fn alloc(&mut self) -> Option<PhysAddr> {
        let p = self.free_list;
        if p != null_mut() {
            unsafe {
                self.free_list = (*p).next;
                for p in slice::from_raw_parts_mut(p as *mut u8, KERNEL_PG_SIZE as usize) {
                    *p = 2;
                }
            }
            trace!("allocator: alloc new page at: 0x{:x}", p as u64);
            Some(p as u64)
        } else {
            None
        }
    }
}
