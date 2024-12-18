use alloc::{sync::Arc, vec};
use core::str;

use log::debug;
use spin::RwLock;
use syscall::SystemCall;

use super::enable_interrupt;
use crate::{
    print,
    proc::{cpu::CPU, exec, schedule, Proc, ProcState},
};

pub fn handle_system_call(proc: Arc<RwLock<Proc>>) {
    let a7 = {
        let mut proc_guard = proc.write();
        let tp = &mut proc_guard.trap_frame;

        debug!("syscall: a0 = {}, a1 = {}, a2 = {}, a7 = {}", tp.a0, tp.a1, tp.a2, tp.a7);

        // Jump the program counter to the next instruction
        tp.epc += 4;

        tp.a7
    };

    unsafe { enable_interrupt() };

    let which: SystemCall = unsafe { core::mem::transmute(a7) };
    let ret = match which {
        SystemCall::Open => handle_syscall_open(proc.clone()),
        SystemCall::Exit => handle_syscall_exit(proc.clone()),
        SystemCall::Exec => handle_syscall_exec(proc.clone()),
        SystemCall::Read => handle_syscall_read(proc.clone()),
        SystemCall::Write => handle_syscall_write(proc.clone()),
        _ => unimplemented!("unimplemented syscall: {:?}", which),
    };

    {
        let _guard = CPU.current().interrupt_manager().enter_critical();
        let mut proc_guard = proc.write();
        proc_guard.trap_frame.a0 = ret as usize;
    }
}

fn handle_syscall_read(proc: Arc<RwLock<Proc>>) -> isize {
    let _guard = CPU.current().interrupt_manager().enter_critical();

    let mut proc_guard = proc.write();
    let tp = &proc_guard.trap_frame;

    let _fd = tp.a0;
    let src = tp.a1;
    let _len = tp.a2;

    // let buffer = vec![1u8; len].into_boxed_slice();
    let buffer = vec![240, 159, 146, 150].into_boxed_slice();
    let page_table = proc_guard.page_table.as_mut();

    if let Ok(_) = page_table.copy_in(src, &buffer) {
        buffer.len() as isize
    } else {
        -1
    }
}

fn handle_syscall_write(proc: Arc<RwLock<Proc>>) -> isize {
    let _guard = CPU.current().interrupt_manager().enter_critical();
    let proc_guard = proc.write();

    let _fd = proc_guard.trap_frame.a0;
    let src = proc_guard.trap_frame.a1;
    let len = proc_guard.trap_frame.a2;

    let mut buffer = vec![0u8; len].into_boxed_slice();
    let page_table = proc_guard.page_table.as_ref();

    if let Ok(_) = page_table.copy_out(src, len, &mut buffer) {
        print!("{}", str::from_utf8(&buffer).unwrap_or("error"));
        0
    } else {
        -1
    }
}

fn handle_syscall_exit(proc: Arc<RwLock<Proc>>) -> isize {
    {
        let _guard = CPU.current().interrupt_manager().enter_critical();
        let mut proc_guard = proc.write();

        debug!("syscall: handling exit, pid: {}", proc_guard.pid);
        proc_guard.state = ProcState::Zombie;
    }
    schedule();
    0
}

fn handle_syscall_open(proc: Arc<RwLock<Proc>>) -> isize {
    let _guard = CPU.current().interrupt_manager().enter_critical();
    let mut proc_guard = proc.write();

    let path_addr = proc_guard.trap_frame.a0;
    let _flags = proc_guard.trap_frame.a1;

    let page_table = proc_guard.page_table.as_ref();

    // match page_table.copy_out_line(path_addr, 512) {
    //     Some(path) => {
    //         let fs = ROOT_FS.get().unwrap();
    //         match fs.get_inode_from_path(&path, &proc.working_dir) {
    //             Some(f) => match proc.open_file(f) {
    //                 Ok(fd) => fd as isize,
    //                 Err(_) => -1,
    //             },
    //             None => -1,
    //         }
    //     }
    //     None => -1,
    // }
    1
}

fn handle_syscall_exec(proc: Arc<RwLock<Proc>>) -> isize {
    let _guard = CPU.current().interrupt_manager().enter_critical();
    let proc_guard = proc.write();

    let path_addr = proc_guard.trap_frame.a0;
    let argv_addr = proc_guard.trap_frame.a1;

    let page_table = proc_guard.page_table.as_ref();

    let path = match page_table.copy_out_line(path_addr, 512) {
        Some(path) => path,
        None => return -1,
    };

    // TODO: pass args
    let argv = vec![""; 0];
    let argc = argv.len();
    match exec(&path, argv.as_slice()) {
        Ok(_) => {
            return argc as isize;
        }
        Err(_) => -1,
    }
}
