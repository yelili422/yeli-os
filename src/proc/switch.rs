use core::arch::global_asm;

use super::context::ContextSnapshot;

global_asm!(include_str!("switch.asm"));

extern "C" {
    pub fn __switch(current_task_cx: *mut ContextSnapshot, next_task_cs: *mut ContextSnapshot);
}
