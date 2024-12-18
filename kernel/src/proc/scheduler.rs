use alloc::{
    collections::{vec_deque::VecDeque, BTreeMap},
    sync::Arc,
};

use log::{debug, info};
use spin::rwlock::RwLock;

pub use super::{
    context::Context,
    proc::{Proc, ProcId, ProcState},
};
use super::{ProcInitFailed, Scheduler};
use crate::{
    fs::{load_initcode, File, InodeFile, ROOT_FS},
    intr::{disable_interrupt, wait_for_interrupt},
    proc::{cpu::CPU, current_proc, set_current_proc, switch_to},
    sync::once_cell::OnceCell,
};

pub struct ProcManager {
    tasks: BTreeMap<ProcId, Arc<RwLock<Proc>>>,
    queue: VecDeque<ProcId>,
}

impl ProcManager {
    pub const fn new() -> Self {
        ProcManager {
            tasks: BTreeMap::new(),
            queue: VecDeque::new(),
        }
    }

    pub fn get(&self, id: &ProcId) -> Option<&Arc<RwLock<Proc>>> {
        self.tasks.get(id)
    }

    pub fn new_proc(
        &mut self,
        src: &[u8],
        args: &[&str],
    ) -> Result<&Arc<RwLock<Proc>>, ProcInitFailed> {
        let proc = Proc::new(src);
        let pid = proc.pid;
        debug!("proc: allocated new task: {}", pid);

        let inserted = self
            .tasks
            .insert(pid, Arc::new(RwLock::new(proc)))
            .is_none();
        if !inserted {
            panic!("proc: reallocated pid: {}", pid);
        }
        self.queue.push_front(pid);

        Ok(self.tasks.get(&pid).unwrap())
    }

    pub fn user_init(&mut self) -> Result<(), ProcInitFailed> {
        info!("Initializing the user_init process...");
        {
            let initcode = load_initcode();
            let proc = self
                .new_proc(&initcode, &[])
                .expect("failed to create init task");
            assert_eq!(proc.read().pid, ProcId::from(0), "The first pid is not 0");
            let mut proc_guard = proc.write();
            proc_guard.state = ProcState::Runnable;
        }
        Ok(())
    }
}

impl Scheduler for ProcManager {
    fn schedule(&mut self) -> Option<ProcId> {
        for id in self.queue.iter() {
            if let Some(proc) = self.tasks.get(id) {
                let mut proc_guard = proc.write();
                if proc_guard.state == ProcState::Runnable {
                    proc_guard.state = ProcState::Running;
                    return Some(*id);
                }
            }
        }
        None
    }
}

// Lazy initialization, after heap allocator initialized.
pub static SCHEDULER: OnceCell<RwLock<ProcManager>> = OnceCell::new();

pub fn schedule() {
    let mut next_context: Option<Context>;
    let interrupt_manager = CPU.current().interrupt_manager();
    loop {
        next_context = {
            let _guard = interrupt_manager.enter_critical();
            let mut scheduler = SCHEDULER.get().unwrap().write();
            if let Some(next_id) = scheduler.schedule() {
                let next_proc = scheduler.get(&next_id).unwrap();

                set_current_proc(next_proc.clone());
                {
                    let next_proc_lock = next_proc.read();
                    Some(next_proc_lock.context.clone())
                }
            } else {
                None
            }
        };

        if next_context.is_some() {
            break;
        }

        if cfg!(debug_assertions) {
            panic!("no runnable process, stopping...");
        }
        wait_for_interrupt();
    }

    let mut current_context = {
        if let Some(proc) = current_proc() {
            let _guard = interrupt_manager.enter_critical();
            let mut proc_guard = proc.write();
            proc_guard.state = ProcState::Runnable;
            proc_guard.context.clone()
        } else {
            Context::default()
        }
    };

    unsafe { disable_interrupt() };

    debug!("scheduler: switching to next process...");
    unsafe { switch_to(&mut current_context, &mut next_context.unwrap()) }
    panic!("unreachable after schedule");
}

pub fn yield_() {
    if let Some(proc) = current_proc() {
        let mut proc_guard = proc.write();
        if proc_guard.state == ProcState::Running {
            proc_guard.state = ProcState::Runnable;
        }
    }
    schedule();
    panic!("after yield");
}

// pub fn sleep() -> ! {
//     if let Some(proc) = current_proc() {
//         let mut proc_guard = proc.write();
//         proc_guard.state = ProcState::Sleeping;
//     }
//     schedule();
// }

pub fn exec(path: &str, args: &[&str]) -> Result<(), ProcInitFailed> {
    debug!("proc: executing {} with args: {:?}...", path, args);

    let _guard = CPU.current().interrupt_manager().enter_critical();
    let fs = ROOT_FS.get().unwrap();
    match fs.get_inode_from_path(path, &fs.root()) {
        Some(exec_inode) => {
            let mut f = InodeFile::open(exec_inode);
            let exec_bytes = f.read_file();

            debug!("exec: loaded executable bytes: {}", exec_bytes.len());
            {
                let mut schedular = SCHEDULER.get().unwrap().write();
                match schedular.new_proc(&exec_bytes, args) {
                    Ok(proc) => {
                        proc.write().state = ProcState::Runnable;
                        Ok(())
                    }
                    Err(err) => Err(err),
                }
            }
        }
        None => return Err(ProcInitFailed::NoSuchFile),
    }
}
