use core::arch::global_asm;

use alloc::{boxed::Box, vec};

use crate::{
    intr::{usertrapret, TrapFrame},
    mem::{
        page::{PTEFlags, PageTable},
        PAGE_SIZE, TRAMPOLINE, TRAPFRAME,
    },
    println, va2pa,
};

pub use self::{backtrace::*, context::Context, task_manager::*};

mod backtrace;
mod context;
mod task_manager;

global_asm!(include_str!("switch.S"));

/// Maximum number of processes.
pub const MAX_PROC: usize = 64;

/// The default kernel stack size.
pub const KSTACK_SIZE: usize = PAGE_SIZE;

/// The default user stack size.
pub const STACK_SIZE: usize = PAGE_SIZE;

// TODO: make a atomic number
pub type ContextId = u32;

pub struct Proc {
    pub pid:        ContextId,
    pub state:      State,
    /// The kernel stack is part of the kernel space. Hence,
    /// it is not directly accessible from a user process.
    pub stack:      Box<[u8]>,
    pub context:    Context,
    pub trap_frame: TrapFrame,
    pub page_table: PageTable,
}

impl Proc {
    pub fn new(pid: ContextId) -> Self {
        // TODO: do we need a guard page ?
        let stack = vec![0; KSTACK_SIZE].into_boxed_slice();

        let trap_frame = TrapFrame::default();

        let mut page_table = PageTable::empty();
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
                va2pa!(&trap_frame as *const _ as usize),
                PAGE_SIZE,
                PTEFlags::R | PTEFlags::W,
            );
        }

        // Set up new context to start executing at `usertrapret`,
        // which returns to user space. Since, we set `sp` to kernel
        // stack temporarily.
        let context = Context {
            ra: usertrapret as usize,
            sp: stack.as_ptr() as usize + stack.len(),
            ..Default::default()
        };

        Proc {
            pid,
            state: State::Runnable,
            stack,
            context,
            trap_frame,
            page_table,
        }
    }

    // pub fn spawn(pid: ContextId, func: extern "C" fn()) -> Self {
    //     let mut proc = Proc::new(pid);

    //     // Initialize kernel stack, push back context.
    //     // let offset = proc.stack.len() - size_of::<usize>();
    //     // unsafe {
    //     //     let stack_top = proc.stack.as_mut_ptr().add(offset);
    //     //     println!("kernel_stack_top -> 0x{:x}", stack_top as usize);
    //     //     *(stack_top as *mut usize) = func as usize;
    //     // }

    //     // let context = Context {
    //     //     ra: usertrapret as usize,
    //     //     sp: stack.as_ptr() as usize + DEFAULT_KSTACK_SIZE,
    //     //     ..Default::default()
    //     // };
    //     // let trap_frame = TrapFrame {
    //     //     epc: 0,
    //     //     sp: user_stack.len(),
    //     //     ..Default::default()
    //     // };

    //     // proc.context.sp = proc.stack.as_ptr() as usize + offset;
    //     proc.context.sp = proc.stack.as_ptr() as usize + proc.stack.len();

    //     proc.context.ra = func as usize;

    //     proc
    // }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum State {
    Sleeping,
    Runnable,
    Running,
    Exited(i32),
}

extern "C" {
    /// Saves/Restores the registers from `Context` and switches
    /// process to other.
    fn switch_to(old: *mut Context, new: *const Context);

    /// The linker identifier of trampoline section.
    fn trampoline();
}

pub fn schedule() -> ! {
    let init_proc_context: *const Context;

    {
        let tm_lock = TASK_MANAGER.read();
        println!(0);

        let init_proc = tm_lock.tasks.get(&0).unwrap();
        {
            let init_proc_lock = init_proc.read();
            init_proc_context = &init_proc_lock.context;
        }
    }

    println!(1);
    unsafe { switch_to(&mut Context::default(), init_proc_context) }

    panic!("unreachable.")
}

pub fn init() {
    {
        let mut task_manager = TASK_MANAGER.write();
        task_manager.user_init();
    }
    backtrace()
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
