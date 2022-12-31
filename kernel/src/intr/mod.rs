/// In this kernel, interrupts/exceptions will be processed below:
/// - syscalls or exceptions from applications of `U` privileged level
/// will be processed by kernel in `S` privileged level.
/// - timer, software, and external interrupts from 'S' privileged level
/// will be process by kernel in `S` privileged level.
///
/// When a interrupt occurs, the same level interrupts will be blocked
/// by default. The hardware will:
/// - Save `sstatus.sie` to `sstatus.spie`, set `sstatus.sie` to 0.
/// It blocks the same levels interrupts.
/// - Restore `sstatus.sie` from `sstatus.spie` after handing interrupt
/// and call `sret` to back to the instruction where interrupted.
use core::{arch::global_asm, panic};

use riscv::register::{
    scause::{self, Interrupt, Trap},
    sie, sstatus, stvec,
    utvec::TrapMode,
};

use self::timer::{set_next_timer, tick};

pub use self::trap::{usertrapret, TrapFrame};

mod timer;
mod trap;

// Import the trap code for user process and kernel process.
global_asm!(include_str!("trampoline.S"));
global_asm!(include_str!("kernelvec.S"));

extern "C" {
    /// The linker identifier of trampoline section.
    fn trampoline();

    /// The linker identifier of `uservec`.
    fn uservec();

    /// The linker identifier of `userret`.
    fn userret(trapframe: usize, satp: usize);

    /// The linker identifier of `kernelvec`.
    fn kernelvec();
}

/// Handles all traps from user or kernel process.
pub fn handle(cause: scause::Scause, _context: &mut TrapFrame) {
    match cause.cause() {
        Trap::Exception(_) => unimplemented!(),
        Trap::Interrupt(Interrupt::SupervisorTimer) => tick(),
        _ => panic!(),
    }
}

pub fn init() {
    // set kernel interrupt handler.
    unsafe { stvec::write(kernelvec as usize, TrapMode::Direct) };

    // enable timer interrupt.
    unsafe {
        sie::set_stimer();
        sstatus::set_sie();
    }
    set_next_timer();
}
