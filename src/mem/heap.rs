use linked_list_allocator::LockedHeap;

pub const KERNEL_HEAP_SIZE: usize = 0x20_0000; // 2M

// Allocate a large block of memory as heap space in .bss segment.
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

pub unsafe fn init() {
    HEAP_ALLOCATOR
        .lock()
        .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, vec::Vec};

    #[test_case]
    fn test_heap_allocate() {
        let v = Box::new(5);
        assert_eq!(*v, 5);
        core::mem::drop(v);

        let mut v = Vec::new();
        for i in 0..10000 {
            v.push(i);
        }
        assert_eq!(v.len(), 10000);
        for (i, value) in v.into_iter().enumerate() {
            assert_eq!(value, i);
        }
    }

    #[test_case]
    fn test_heap_in_bss() {
        extern "C" {
            fn __bss_start();
            fn __bss_end();
        }
        let bss_range = __bss_start as usize .. __bss_end as usize;
        let a = Box::new(1);
        assert_eq!(*a, 1);
        assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
        drop(a);

        let mut v: Vec<usize> = Vec::new();
        for i in 0..500 {
            v.push(i);
        }
        for i in 0..500 {
            assert_eq!(v[i], i);
        }
        assert!(bss_range.contains(&(v.as_ptr() as usize)));
        drop(v);
    }
}
