mod sbi;

use core::arch::asm;

pub use sbi::{console_getchar, console_putchar, set_timer, shutdown};

fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!("ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id,
            options(nostack)
        )
    }
    ret
}

pub const SYSCALL_WRITE: usize = 64;
pub const SYSCALL_TIME: usize = 169;

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_time() -> isize {
    syscall(SYSCALL_TIME, [0; 3])
}
