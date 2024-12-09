use core::arch::global_asm;

use log::{debug, info};
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub use self::{backtrace::*, context::Context, task::*, task_list::*};
use crate::{mem::PAGE_SIZE, println};

mod backtrace;
mod context;
mod task;
mod task_list;

global_asm!(include_str!("switch.S"));

/// Maximum number of processes.
pub const MAX_PROC: u64 = 64;

/// The default kernel stack size.
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 2;

/// The default user stack size.
pub const USER_STACK_SIZE: usize = PAGE_SIZE * 2;

pub static TASKS: RwLock<TaskList> = RwLock::new(TaskList::new());

pub fn tasks() -> RwLockReadGuard<'static, TaskList> {
    TASKS.read()
}

pub fn tasks_mut() -> RwLockWriteGuard<'static, TaskList> {
    TASKS.write()
}

extern "C" {
    /// Saves/Restores the registers from `Context` and switches
    /// process to other.
    fn switch_to(old: *mut Context, new: *const Context);
}

pub fn schedule() -> ! {
    let init_proc_context: *const Context;
    {
        let tasks = tasks();
        let init_proc = tasks.get(&0).unwrap();
        {
            let init_proc_lock = init_proc.read();
            init_proc_context = &init_proc_lock.context;
        }
    }

    info!("switching to next process...");
    unsafe { switch_to(&mut Context::default(), init_proc_context) }

    panic!("unreachable.")
}

pub fn init() {
    info!("Initializing processes...");
    {
        let mut tasks = tasks_mut();
        tasks.user_init();
    }
    // backtrace()
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
