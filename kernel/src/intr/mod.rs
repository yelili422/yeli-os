use alloc::sync::Arc;
use core::arch::global_asm;

use log::{debug, info};
use plic::handle_plic;
use riscv::{
    asm::wfi,
    interrupt::{supervisor::Interrupt, Exception},
    register::{
        scause::{self, Trap},
        sie, sstatus, stval,
        stvec::{self, TrapMode},
    },
    ExceptionNumber, InterruptNumber,
};
use spin::RwLock;
use syscall::handle_system_call;

use self::timer::{set_next_timer, tick};
pub use self::{
    sbi::shutdown,
    trap::{usertrapret, TrapFrame},
};
use crate::proc::{yield_, Proc};

pub mod guard;
pub mod plic;
mod sbi;
mod syscall;
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
pub unsafe fn handle(cause: scause::Scause, proc: Option<Arc<RwLock<Proc>>>) {
    let stval = stval::read();
    debug!("intr: handling interrupt...");
    {
        match cause.cause() {
            Trap::Exception(exception) => match Exception::from_number(exception) {
                Err(err) => panic!("{}", err),
                Ok(Exception::LoadPageFault) | Ok(Exception::StorePageFault) => {
                    panic!("pagefault: bad addr = {:#x}", stval,);
                }
                Ok(Exception::UserEnvCall) => handle_system_call(proc.expect("no process")),
                Ok(e) => unimplemented!("unimplemented exception {:?}", e),
            },
            Trap::Interrupt(intr) => match Interrupt::from_number(intr) {
                Err(err) => panic!("{}", err),
                Ok(Interrupt::SupervisorTimer) => {
                    tick();
                    yield_();
                }
                Ok(Interrupt::SupervisorExternal) => handle_plic(),
                Ok(i) => unimplemented!("unimplemented interrupt {:?}", i),
            },
        }
    }
}

pub fn init() {
    info!("Initializing interrupt handlers...");

    unsafe {
        // set kernel interrupt handler.
        stvec::write(kernelvec as usize, TrapMode::Direct);

        // enable timer interrupt.
        sie::set_stimer();

        // enable PLIC interrupts
        // plic_init();

        enable_interrupt();
        enable_external_interrupt();
    }
    set_next_timer();
}

#[inline(always)]
pub unsafe fn disable_interrupt() {
    sstatus::clear_sie();
}

#[inline(always)]
pub unsafe fn enable_interrupt() {
    sstatus::set_sie();
}

#[inline(always)]
unsafe fn enable_external_interrupt() {
    sie::set_sext();
}

#[inline(always)]
unsafe fn disable_external_interrupt() {
    sie::clear_sext();
}

#[inline(always)]
pub fn wait_for_interrupt() {
    wfi();
}
