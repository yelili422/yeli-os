use core::arch::{asm, global_asm};

use log::info;
use plic::{handle_plic, plic_init};
use riscv::{
    interrupt::{supervisor::Interrupt, Exception},
    register::{
        scause::{self, Trap},
        sie, sstatus, stval,
        stvec::{self, TrapMode},
    },
    ExceptionNumber, InterruptNumber,
};

use self::timer::{set_next_timer, tick};
pub use self::trap::{usertrapret, TrapFrame};

pub mod plic;
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
pub unsafe fn handle(cause: scause::Scause, context: &mut TrapFrame) {
    disable_supervisor_external_interrupt();
    disable_supervisor_interrupt();

    let stval = stval::read();
    match cause.cause() {
        Trap::Exception(exception) => match Exception::from_number(exception) {
            Err(err) => panic!("{}", err),
            Ok(Exception::LoadPageFault) | Ok(Exception::StorePageFault) => {
                panic!("pagefault: bad addr = {:#x}, instruction = {:#x}", stval, context.epc,);
            }
            Ok(e) => unimplemented!("{:?}", e),
        },
        Trap::Interrupt(intr) => match Interrupt::from_number(intr) {
            Err(err) => panic!("{}", err),
            Ok(Interrupt::SupervisorTimer) => tick(),
            Ok(Interrupt::SupervisorExternal) => handle_plic(),
            Ok(e) => unimplemented!("{:?}", e),
        },
    }

    enable_supervisor_interrupt();
    enable_supervisor_external_interrupt();
}

pub fn init() {
    info!("Initializing interrupt handlers...");

    unsafe {
        // set kernel interrupt handler.
        stvec::write(kernelvec as usize, TrapMode::Direct);

        // enable timer interrupt.
        sie::set_stimer();

        // enable PLIC interrupts
        plic_init();

        enable_supervisor_interrupt();
        enable_supervisor_external_interrupt();
    }
    set_next_timer();
}

#[inline(always)]
pub fn cpu_id() -> usize {
    // let id: usize;
    // unsafe { asm!("mv {}, tp", out(reg) id) };
    // id
    0
}

#[inline(always)]
pub fn set_cpu_id(id: usize) {
    unsafe { asm!("mv tp, {}", in(reg) id) };
}

#[inline(always)]
unsafe fn disable_supervisor_interrupt() {
    sstatus::clear_sie();
}

#[inline(always)]
unsafe fn enable_supervisor_interrupt() {
    sstatus::set_sie();
}

#[inline(always)]
unsafe fn enable_supervisor_external_interrupt() {
    sie::set_sext();
}

#[inline(always)]
unsafe fn disable_supervisor_external_interrupt() {
    sie::clear_sext();
}
