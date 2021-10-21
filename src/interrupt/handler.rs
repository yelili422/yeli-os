use log::info;
use riscv::register::{
    scause::{Exception, Interrupt, Scause, Trap},
    stvec,
};

use super::{context::Context, timer};

global_asm!(include_str!("./interrupt.s"));

pub fn init() {
    extern "C" {
        fn __interrupt();
    }
    unsafe {
        stvec::write(__interrupt as usize, stvec::TrapMode::Direct);
    }
}

#[no_mangle]
pub fn handle_interrupt(context: &mut Context, scause: Scause, stval: usize) {
    match scause.cause() {
        Trap::Exception(Exception::Breakpoint) => breakpoint(context),
        Trap::Interrupt(Interrupt::SupervisorTimer) => supervisor_timer(context),
        _ => fault(context, scause, stval),
    }
}

fn breakpoint(context: &mut Context) {
    info!("Breakpoint at 0x{:x}", context.sepc);
    // 继续执行，其中 `sepc` 增加 2 字节，以跳过当前这条 `ebreak` 指令
    context.sepc += 2;
}

fn supervisor_timer(_context: &mut Context) {
    timer::tick();
}

fn fault(context: &mut Context, scause: Scause, stval: usize) {
    panic!(
        "Unresolved interrupt: {:?}\n{:x?}\nstval: {:x}",
        scause.cause(),
        context,
        stval
    );
}
