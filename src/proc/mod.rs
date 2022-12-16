use core::arch::global_asm;

pub use self::{context::Context, proc::*, task_manager::*};

mod context;
mod proc;
mod task_manager;

global_asm!(include_str!("switch.S"));
global_asm!(include_str!("trampoline.S"));

extern "C" {
    /// Saves/Restores the registers from `Context` and switches
    /// process to other.
    fn switch_to(old: *mut Context, new: *const Context);

    /// The linker identifier of trampoline section.
    pub static trampoline: u8;
}

pub const MAX_PROC: u32 = 64;

pub fn init() {
    TASK_MANAGER.user_init();
    TASK_MANAGER.schedule();
}
