use alloc::boxed::Box;

use crate::{
    intr::TrapFrame,
    mem::{
        page::{PTEFlags, PageTable},
        PAGE_SIZE, TRAMPOLINE, TRAPFRAME,
    },
    va2pa,
};

use super::{trampoline, Context};

pub type TaskId = u64;

pub struct Task {
    pub pid:          TaskId,
    pub state:        State,
    /// The kernel stack is part of the kernel space. Hence,
    /// it is not directly accessible from a user process.
    pub kernel_stack: Option<Box<[u8]>>,
    pub context:      Context,
    pub trap_frame:   TrapFrame,
    pub page_table:   Option<PageTable>,
}

impl Task {
    pub fn new(pid: TaskId) -> Self {
        Task {
            pid,
            state: State::Blocked,
            kernel_stack: None,
            context: Context::default(),
            trap_frame: TrapFrame::default(),
            page_table: None,
        }
    }

    pub fn set_user_page_table(&mut self, mut page_table: PageTable) {
        unsafe {
            // Map trampoline code (for system call return) at the hightest
            // user virtual address. Only the supervisor uses it, on the
            // way to/from user space, so not PTE::U.
            page_table.map(
                TRAMPOLINE,
                va2pa!(trampoline as usize),
                PAGE_SIZE,
                PTEFlags::R | PTEFlags::X,
            );

            // Map the trap frame just below TRAMPOLINE,
            // for the trampoline.S.
            page_table.map(
                TRAPFRAME,
                va2pa!(&self.trap_frame as *const _ as usize),
                PAGE_SIZE,
                PTEFlags::R | PTEFlags::W,
            );
        }
        self.page_table = Some(page_table);
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum State {
    Sleeping,
    Runnable,
    Running,
    Blocked,
    Exited(i32),
}
