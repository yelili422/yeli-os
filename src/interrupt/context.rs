
use riscv::register::sstatus::Sstatus;


#[repr(C)]
#[derive(Debug)]
pub struct Context {
    pub x: [usize; 32], // 32 general purpose registers
    pub sstatus: Sstatus,
    pub sepc: usize,
}
