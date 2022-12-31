use core::arch::asm;

use log::info;

use crate::println;

use super::TASK_MANAGER;

#[inline(always)]
fn r_fp() -> usize {
    let mut x: usize;
    unsafe {
        asm!(
            "mv {ret}, s0",
            ret = out(reg) x,
            options(nostack)
        )
    }
    x
}

pub fn backtrace() {
    // let fp = r_fp();
    {
        let tm = TASK_MANAGER.read();
        {
            let proc_lock = tm.current().expect("get current process failed.").read();

            // println!("{:?}", &proc_lock.stack);
        }
    }
}
