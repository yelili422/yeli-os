use alloc::sync::Arc;
use core::{arch::asm, cell::UnsafeCell};

use spin::RwLock;

use super::Proc;
use crate::intr::guard::InterruptManager;

const N_CPU: usize = 1;

pub struct Cpu {
    proc:         Option<Arc<RwLock<Proc>>>,
    intr_manager: InterruptManager,
}

impl Cpu {
    pub const fn new() -> Self {
        Self {
            proc:         None,
            intr_manager: InterruptManager::new(),
        }
    }

    pub fn current_proc(&self) -> Option<Arc<RwLock<Proc>>> {
        self.proc.clone()
    }

    pub fn set_current_proc(&mut self, proc: Arc<RwLock<Proc>>) {
        self.proc = Some(proc);
    }

    pub fn interrupt_manager(&mut self) -> &mut InterruptManager {
        &mut self.intr_manager
    }
}

unsafe impl Sync for Cpu {}

pub struct CpuManager {
    cpus: UnsafeCell<[Cpu; N_CPU]>,
}

impl CpuManager {
    pub const fn new() -> Self {
        Self {
            // SAFETY: `cpu` is per-CPU data, so it is thread-safe.
            cpus: UnsafeCell::new([Cpu::new(); N_CPU]),
        }
    }

    pub fn current(&self) -> &'static mut Cpu {
        unsafe { &mut (*self.cpus.get())[cpu_id()] }
    }
}

unsafe impl Sync for CpuManager {}

pub static CPU: CpuManager = CpuManager::new();

#[inline(always)]
pub fn cpu_id() -> usize {
    let id: usize;
    unsafe { asm!("mv {}, tp", out(reg) id) };
    id
}

#[inline(always)]
pub fn set_cpu_id(id: usize) {
    unsafe { asm!("mv tp, {}", in(reg) id) };
}
