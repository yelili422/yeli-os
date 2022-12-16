/// Saves registers for thread switching.
///
/// Only saves callee-saved registers, caller-saved registers are saved
/// on the stack (if needed).
///
/// It doesn't save the program counter. Instead, it saves the `ra`
/// register, which hold the return address from which `switch_to`
/// was called.
/// For example, `sched` called `switch_to` to switch to `cpu->scheduler`,
/// the per-CPU scheduler context. When the `switch_to` we have been
/// tracing returns, it returns not to `sched` but to `scheduler`, and
/// its stack pointer points at the current CPU’s scheduler stack.
#[repr(C)]
#[derive(Default)]
pub struct Context {
    pub ra: u64,
    pub sp: u64,

    // callee-saved
    pub s0: u64,
    pub s1: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
}
