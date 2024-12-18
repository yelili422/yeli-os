use core::fmt;

use log::{debug, trace};
use riscv::register::{scause, sepc, sstatus, stvec};

use super::{handle, kernelvec};
use crate::{
    intr::{disable_interrupt, trampoline, userret, uservec},
    mem::{allocator::FromRawPage, page::current_page_table, TRAMPOLINE, TRAP_FRAME},
    proc::current_proc,
};

#[derive(Default)]
#[repr(C, align(4096))]
pub struct TrapFrame {
    /*   0 */ pub kernel_satp:   usize, // kernel page table
    /*   8 */ pub kernel_sp:     usize, // top of process's kernel stack
    /*  16 */ pub kernel_trap:   usize, // usertrap()
    /*  24 */ pub epc:           usize, // saved user program counter
    /*  32 */ pub kernel_hartid: usize, // saved kernel tp
    /*  40 */ pub ra:            usize,
    /*  48 */ pub sp:            usize,
    /*  56 */ pub gp:            usize,
    /*  64 */ pub tp:            usize,
    /*  72 */ pub t0:            usize,
    /*  80 */ pub t1:            usize,
    /*  88 */ pub t2:            usize,
    /*  96 */ pub s0:            usize,
    /* 104 */ pub s1:            usize,
    /* 112 */ pub a0:            usize,
    /* 120 */ pub a1:            usize,
    /* 128 */ pub a2:            usize,
    /* 136 */ pub a3:            usize,
    /* 144 */ pub a4:            usize,
    /* 152 */ pub a5:            usize,
    /* 160 */ pub a6:            usize,
    /* 168 */ pub a7:            usize,
    /* 176 */ pub s2:            usize,
    /* 184 */ pub s3:            usize,
    /* 192 */ pub s4:            usize,
    /* 200 */ pub s5:            usize,
    /* 208 */ pub s6:            usize,
    /* 216 */ pub s7:            usize,
    /* 224 */ pub s8:            usize,
    /* 232 */ pub s9:            usize,
    /* 240 */ pub s10:           usize,
    /* 248 */ pub s11:           usize,
    /* 256 */ pub t3:            usize,
    /* 264 */ pub t4:            usize,
    /* 272 */ pub t5:            usize,
    /* 280 */ pub t6:            usize,
    pub padding:       [usize; 26],
}

impl FromRawPage for TrapFrame {}

impl fmt::Display for TrapFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TrapFrame {{\n")?;
        write!(f, "  kernel_satp:   {:#x}\n", self.kernel_satp)?;
        write!(f, "  kernel_sp:     {:#x}\n", self.kernel_sp)?;
        write!(f, "  kernel_trap:   {:#x}\n", self.kernel_trap)?;
        write!(f, "  epc:           {:#x}\n", self.epc)?;
        write!(f, "  kernel_hartid: {:#x}\n", self.kernel_hartid)?;
        write!(f, "  ra:            {:#x}\n", self.ra)?;
        write!(f, "  sp:            {:#x}\n", self.sp)?;
        write!(f, "  gp:            {:#x}\n", self.gp)?;
        write!(f, "  tp:            {:#x}\n", self.tp)?;
        write!(f, "  t0:            {:#x}\n", self.t0)?;
        write!(f, "  t1:            {:#x}\n", self.t1)?;
        write!(f, "  t2:            {:#x}\n", self.t2)?;
        write!(f, "  s0:            {:#x}\n", self.s0)?;
        write!(f, "  s1:            {:#x}\n", self.s1)?;
        write!(f, "  a0:            {:#x}\n", self.a0)?;
        write!(f, "  a1:            {:#x}\n", self.a1)?;
        write!(f, "  a2:            {:#x}\n", self.a2)?;
        write!(f, "  a3:            {:#x}\n", self.a3)?;
        write!(f, "  a4:            {:#x}\n", self.a4)?;
        write!(f, "  a5:            {:#x}\n", self.a5)?;
        write!(f, "  a6:            {:#x}\n", self.a6)?;
        write!(f, "  a7:            {:#x}\n", self.a7)?;
        write!(f, "  s2:            {:#x}\n", self.s2)?;
        write!(f, "  s3:            {:#x}\n", self.s3)?;
        write!(f, "  s4:            {:#x}\n", self.s4)?;
        write!(f, "  s5:            {:#x}\n", self.s5)?;
        write!(f, "  s6:            {:#x}\n", self.s6)?;
        write!(f, "}}")?;
        Ok(())
    }
}

/// Handles interrupt, exception or system call from user space.
#[no_mangle]
pub unsafe fn usertrap() {
    if sstatus::read().spp() == sstatus::SPP::Supervisor {
        panic!("usertrap: not from user mode");
    }

    // Send syscalls, interrupts, and exceptions to kernelvec
    stvec::write(kernelvec as usize, stvec::TrapMode::Direct);

    {
        let proc_lock = current_proc().expect("usertrap: failed to get current process");
        {
            // Acquire the process' write lock to modify its trap frame.
            // It is ok because we are in usertrap(), which is called once
            // in the user space. It is not nested.
            let mut proc = proc_lock.write();
            // Save user program counter.
            proc.trap_frame.epc = sepc::read();
        }
        handle(scause::read(), Some(proc_lock));
    }

    usertrapret();
}

/// Returns to user space when `usertrap` is done.
#[no_mangle]
pub unsafe fn usertrapret() {
    let satp: usize;

    // We're about to switch the destination of traps from `kerneltrap()`
    // to `usertrap()`, so turn off interrupts until we're back in
    // user space, where `usertrap()` is correct.
    disable_interrupt();

    // Send syscalls, interrupts, and exceptions to trampoline.S
    let entry = TRAMPOLINE + (uservec as usize - trampoline as usize);
    stvec::write(entry, stvec::TrapMode::Direct);

    {
        let current_task = current_proc().expect("usertrapret: failed to get current process");
        let mut proc = current_task.write();

        // Set up trapframe values that `uservec` will need when the
        // process next re-enters the kernel.
        let kernel_stack = proc.kernel_stack.as_ref();
        let kernel_stack_sp = kernel_stack.as_ptr() as usize + kernel_stack.len();
        let trap_frame = &mut proc.trap_frame;

        trap_frame.kernel_satp = current_page_table();
        trap_frame.kernel_sp = kernel_stack_sp;
        trap_frame.kernel_trap = usertrap as usize;

        trace!("usertrapret: trap_frame: {}", trap_frame);

        // Set up the registers that trampoline.S's `sret` will use
        // to get the usr space.

        // Set S Previous Privilege mode to User.
        sstatus::set_spp(sstatus::SPP::User);
        // Enable interrupts in user mode.
        sstatus::set_spie();

        // Set S Exception Program Counter to the saved user pc.
        sepc::write(proc.trap_frame.epc);

        satp = proc.page_table.as_ref().make_satp();
        debug!("usertrapret: user_page_table satp: 0x{:x}", satp);
    }

    // Jump to trampoline.S, which switches to the user page table,
    // restores user registers, and switches to user mode with `sret`.
    let trampoline_userret = TRAMPOLINE + (userret as usize - trampoline as usize);
    let userret_virt: extern "C" fn(usize, usize) -> ! =
        core::mem::transmute(trampoline_userret as usize);
    userret_virt(TRAP_FRAME, satp);
}

#[no_mangle]
pub unsafe fn kerneltrap() {
    handle(scause::read(), current_proc());
}
