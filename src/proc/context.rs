
#[repr(C)]
pub struct ContextSnapshot {
    ra: usize,
    sp: usize,
    s: [usize; 12], // s0 ~ s11
}

impl ContextSnapshot {
    pub fn empty() -> Self {
        return ContextSnapshot { ra: 0, sp: 0, s: [0; 12] }
    }
}

pub struct Context {}

impl Context {
    pub unsafe fn switch(&mut self, target: &mut ContextSnapshot) {
        todo!()
        // __switch(self, target);
    }

    pub fn init() {

    }
}
