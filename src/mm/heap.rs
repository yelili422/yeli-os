use linked_list_allocator::LockedHeap;

const KERNEL_HEAP_SIZE: usize = 0x80_0000; // 8M

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init() {
    unsafe {
        ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn heap_allocate_test() {
        use alloc::boxed::Box;
        use alloc::vec::Vec;

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
}
