pub struct SlabAllocator {}

impl SlabAllocator {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn alloc(&self, size: usize) -> usize {
        0
    }
}
