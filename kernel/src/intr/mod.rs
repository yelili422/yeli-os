use core::arch::global_asm;

use log::info;
use riscv::{
    interrupt::supervisor::Interrupt,
    register::{
        scause::{self, Trap},
        sie, sstatus,
        stvec::{self, TrapMode},
    },
    InterruptNumber,
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
    pub fn trampoline();

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
        Trap::Exception(_expt) => unimplemented!(),
        Trap::Interrupt(intr) => match Interrupt::from_number(intr).unwrap() {
            Interrupt::SupervisorTimer => tick(),
            _ => unimplemented!(),
        },
    }
}

pub fn init() {
    info!("Initializing interrupt handlers...");
    // set kernel interrupt handler.
    unsafe { stvec::write(kernelvec as usize, TrapMode::Direct) };

    // enable timer interrupt.
    unsafe {
        sie::set_stimer();
        sstatus::set_sie();
    }
    set_next_timer();
}
