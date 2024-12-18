#![no_std]

use core::arch::asm;

fn syscall(id: SystemCall, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!("ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id as usize,
            options(nostack)
        )
    }
    ret
}

#[derive(Debug)]
#[repr(usize)]
pub enum SystemCall {
    Exit = 2,
    Exec = 11,
    Open = 56,
    Read = 63,
    Write = 64,
    Time = 169,
}

pub fn exec(path: &str, argv: &[&str]) -> isize {
    syscall(SystemCall::Exec, [path.as_ptr() as usize, argv.as_ptr() as usize, 0])
}

pub fn open(path: &str, flags: usize) -> isize {
    syscall(SystemCall::Open, [path.as_ptr() as usize, flags, 0])
}

pub fn read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall(SystemCall::Read, [fd, buffer.as_mut_ptr() as usize, buffer.len()])
}

pub fn write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SystemCall::Write, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn time() -> isize {
    syscall(SystemCall::Time, [0; 3])
}

pub fn exit(exit_code: usize) -> ! {
    syscall(SystemCall::Exit, [exit_code, 0, 0]);
    panic!("unreachable: after exit");
}
