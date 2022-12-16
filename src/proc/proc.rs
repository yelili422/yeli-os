use core::mem::size_of;

use alloc::{boxed::Box, vec};
use spin::Spin;

use crate::{
    addr,
    mem::{page::PageTable, PAGE_SIZE},
    println,
    proc::{context::Context, trampoline},
};

pub type ContextId = u32;

pub struct Proc {
    lock: Spin,
    pub pid: u32,
    pub state: State,
    user_stack: Box<[u8]>,
    /// The kernel stack is part of the kernel space. Hence,
    /// it is not directly accessible from a user process.
    kernel_stack: Box<[u8]>,
    pub context: Context,
    pub trap_frame: TrapFrame,
    page_table: PageTable,
}

impl Proc {
    pub fn new(pid: ContextId) -> Self {
        const KSTACK_SIZE: u64 = 65536;
        const STACK_SIZE: u64 = PAGE_SIZE * 2;

        // TODO: do we need a guard page ?
        let kernel_stack = vec![0; KSTACK_SIZE as usize].into_boxed_slice();
        let user_stack = vec![0; STACK_SIZE as usize].into_boxed_slice();

        let context = Context {
            ra: todo!(), // TODO: return by trap
            sp: kernel_stack.as_ptr() as u64 + STACK_SIZE,
            ..Default::default()
        };
        let trap_frame = TrapFrame::default();

        let page_table =
            PageTable::init_proc(addr!(trampoline), &trap_frame as *const _ as u64, &user_stack);

        Proc {
            lock: Spin,
            pid,
            state: State::Runnable,
            user_stack,
            kernel_stack,
            context,
            trap_frame,
            page_table,
        }
    }

    pub fn from_fn(pid: ContextId, func: extern "C" fn()) -> Self {
        let mut proc = Proc::new(pid);

        // Initialize kernel stack, push back context.
        let offset = proc.kernel_stack.len() - size_of::<usize>();
        unsafe {
            let func_ptr = proc.kernel_stack.as_mut_ptr().add(offset);
            println!("kernel_stack_top -> 0x{:x}", func_ptr as u64);
            *(func_ptr as *mut usize) = func as usize;
        }

        proc
    }
}

#[repr(C)]
#[derive(Default)]
pub struct TrapFrame {
    /*   0 */ pub kernel_satp: u64, // kernel page table
    /*   8 */ pub kernel_sp: u64, // top of process's kernel stack
    /*  16 */ pub kernel_trap: u64, // usertrap()
    /*  24 */ pub epc: u64, // saved user program counter
    /*  32 */ pub kernel_hartid: u64, // saved kernel tp
    /*  40 */ pub ra: u64,
    /*  48 */ pub sp: u64,
    /*  56 */ pub gp: u64,
    /*  64 */ pub tp: u64,
    /*  72 */ pub t0: u64,
    /*  80 */ pub t1: u64,
    /*  88 */ pub t2: u64,
    /*  96 */ pub s0: u64,
    /* 104 */ pub s1: u64,
    /* 112 */ pub a0: u64,
    /* 120 */ pub a1: u64,
    /* 128 */ pub a2: u64,
    /* 136 */ pub a3: u64,
    /* 144 */ pub a4: u64,
    /* 152 */ pub a5: u64,
    /* 160 */ pub a6: u64,
    /* 168 */ pub a7: u64,
    /* 176 */ pub s2: u64,
    /* 184 */ pub s3: u64,
    /* 192 */ pub s4: u64,
    /* 200 */ pub s5: u64,
    /* 208 */ pub s6: u64,
    /* 216 */ pub s7: u64,
    /* 224 */ pub s8: u64,
    /* 232 */ pub s9: u64,
    /* 240 */ pub s10: u64,
    /* 248 */ pub s11: u64,
    /* 256 */ pub t3: u64,
    /* 264 */ pub t4: u64,
    /* 272 */ pub t5: u64,
    /* 280 */ pub t6: u64,
}

#[derive(PartialEq, Eq)]
pub enum State {
    Sleeping,
    Runnable,
    Running,
    Exited(i32),
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
