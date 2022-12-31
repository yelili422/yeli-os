/// Saves registers for thread switching.
///
/// Only saves callee-saved registers, caller-saved registers are saved
/// on the stack (if needed).
///
/// It doesn't save the program counter. Instead, it saves the `ra`
/// register, which hold the return address from which `switch_to`
/// was called.
/// For example, `sched` called `switch` to switch to `cpu->scheduler`,
/// the per-CPU scheduler context. When the `switch_to` we have been
/// tracing returns, it returns not to `sched` but to `scheduler`, and
/// its stack pointer points at the current CPU’s scheduler stack.
#[repr(C)]
#[derive(Default)]
pub struct Context {
    pub ra: usize,
    pub sp: usize,

    // callee-saved
    pub s0:  usize,
    pub s1:  usize,
    pub s2:  usize,
    pub s3:  usize,
    pub s4:  usize,
    pub s5:  usize,
    pub s6:  usize,
    pub s7:  usize,
    pub s8:  usize,
    pub s9:  usize,
    pub s10: usize,
    pub s11: usize,
}
