use alloc::sync::Arc;
use core::arch::global_asm;

use cpu::CPU;
use log::info;
use spin::rwlock::RwLock;

pub use self::{
    context::Context,
    proc::{Proc, ProcId, ProcState},
    scheduler::*,
};

mod context;
pub mod cpu;
mod proc;
mod scheduler;

global_asm!(include_str!("switch.S"));

pub fn current_proc() -> Option<Arc<RwLock<Proc>>> {
    let cpu = CPU.current();
    cpu.current_proc()
}

fn set_current_proc(proc: Arc<RwLock<Proc>>) {
    let cpu = CPU.current();
    cpu.set_current_proc(proc);
}

extern "C" {
    /// Saves/Restores the registers from `Context` and switches
    /// process to other.
    fn switch_to(old: *mut Context, new: *mut Context);
}

pub fn init() {
    info!("Initializing processes...");
    let mut procs = SCHEDULER
        .get_or_init(|| RwLock::new(ProcManager::new()))
        .write();
    procs.user_init().unwrap();
}

pub trait Scheduler {
    fn schedule(&mut self) -> Option<ProcId>;
}

#[derive(Debug)]
pub enum ProcInitFailed {
    NoSuchFile,
}

#[cfg(test)]
mod tests {

    // extern fn spawned_task() {
    //     println!("Spawn new task finished");
    // }

    // #[test_case]
    // fn test_init_task() {
    //     Proc::spawn(spawned_task);
    // }

    // #[no_mangle]
    // extern "C" fn switch(current: &mut Proc, next: &mut Proc) {
    //     println!("thread 1");
    // }

    // #[test_case]
    // fn test_thread_switch() {}
}
