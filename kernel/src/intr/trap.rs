use riscv::register::{scause, sepc, sstatus, stvec};

use super::handle;
use crate::{
    intr::{trampoline, userret, uservec},
    mem::{TRAMPOLINE, TRAPFRAME},
    println,
    proc::TASKS,
};

#[repr(C)]
#[derive(Default)]
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
}

/// Handles interrupt, exception or system call from user space.
#[no_mangle]
pub fn usertrap() {
    if sstatus::read().spp() == sstatus::SPP::Supervisor {
        panic!("usertrap: not from user mode");
    }

    // TODO:
    // stvec::write(kernelvec)

    {
        let lock = TASKS.write();
        let proc = lock
            .current()
            .expect("usertrap: failed to get current process");
        {
            let mut proc_lock = proc.write();

            // Save user program counter.
            proc_lock.trap_frame.epc = sepc::read();

            handle(scause::read(), &mut proc_lock.trap_frame);
        }
    }
}

/// Returns to user space when `usertrap` is done.
#[no_mangle]
pub unsafe fn usertrapret() {
    let satp: usize;

    {
        let tasks = TASKS.write();

        // We're about to switch the destination of traps from `kerneltrap()`
        // to `usertrap()`, so turn off interrupts until we're back in
        // user space, where `usertrap()` is correct.
        sstatus::clear_sie();

        // Send syscalls, interrupts, and exceptions to trampoline.S
        let entry = TRAMPOLINE + (uservec as usize - trampoline as usize);
        stvec::write(entry, stvec::TrapMode::Direct);

        {
            let current_task = match tasks.current() {
                Ok(current_task) => current_task,
                Err(_) => panic!("get current process failed."),
            };
            let proc = current_task.write();

            // // Set up trapframe values that `uservec` will need when the
            // // process next re-enters the kernel.
            // let stack = proc.kernel_stack.as_ref();
            // proc.trap_frame = TrapFrame {
            //     kernel_satp: current_page_table(), // kernel page table.
            //     kernel_sp: stack.as_ptr() as usize + stack.len(), // kernel stack
            //     kernel_trap: usertrap as usize,
            //     ..Default::default()
            // };

            // Set up the registers that trampoline.S's `sret` will use
            // to get the usr space.

            // Set S Previous Privilege mode to User.
            sstatus::set_spp(sstatus::SPP::User);
            // Enable interrupts in user mode.
            sstatus::set_spie();

            // Set S Exception Program Counter to the saved user pc.
            sepc::write(proc.trap_frame.epc);

            satp = match proc.page_table.as_ref() {
                Some(pt) => {
                    println!("enable page table: {}", pt);
                    pt.make_satp()
                }
                None => panic!("invalid process"),
            }
        }
    }
    println!(4);

    // Jump to trampoline.S, which switches to the user page table,
    // restores user registers, and switches to user mode with `sret`.
    let trampoline_userret = TRAMPOLINE + (userret as usize - trampoline as usize);
    println!("userret: 0x{:x}", trampoline_userret as usize);
    let userret_virt: extern "C" fn(usize, usize) -> ! =
        core::mem::transmute(trampoline_userret as usize);
    userret_virt(TRAPFRAME, satp);
}

#[no_mangle]
pub fn kerneltrap() {
    {
        let lock = TASKS.write();
        let proc = lock
            .current()
            .expect("usertrap: failed to get current process");
        {
            let mut proc_lock = proc.write();

            handle(scause::read(), &mut proc_lock.trap_frame);
        }
    }
}
