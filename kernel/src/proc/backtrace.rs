use core::arch::asm;

// use super::tasks;

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

// pub fn backtrace() {
//     // let fp = r_fp();
//     {
//         let tasks = tasks();
//         let current = tasks.current().expect("get current process failed.").read();

//         // println!("{:?}", &current.stack);
//     }
// }
