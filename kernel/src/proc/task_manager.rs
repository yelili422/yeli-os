use alloc::{collections::BTreeMap, sync::Arc};
use spin::RwLock;

use crate::proc::{ContextId, Proc, MAX_PROC};

use super::State;

pub struct TaskManager {
    pub tasks: BTreeMap<ContextId, Arc<RwLock<Proc>>>,
    next_id:   u32,
}

impl TaskManager {
    pub const fn new() -> Self {
        TaskManager {
            tasks:   BTreeMap::new(),
            next_id: 0,
        }
    }

    pub fn alloc_pid(&mut self) -> ContextId {
        self.next_id += 1;
        self.next_id - 1
    }

    pub fn user_init(&mut self) {
        let pid = self.alloc_pid();
        assert_eq!(pid, 0, "The first pid is not 0");

        let mut proc = Proc::new(pid);

        // Prepare for the very first "return" form kernel to user.
        proc.trap_frame.epc = 0; // user program counter
        proc.trap_frame.sp = proc.stack.len(); // user stack pointer

        // TODO: Copy init code to user space.

        proc.state = State::Runnable;

        self.tasks.insert(pid, Arc::new(RwLock::new(proc)));
    }

    pub fn spawn(&self, _func: extern "C" fn()) -> Result<&Arc<RwLock<Proc>>, ()> {
        if self.next_id as usize > MAX_PROC {}
        todo!()
    }

    pub fn current(&self) -> Result<&Arc<RwLock<Proc>>, ()> {
        // TODO:
        self.tasks.get(&0).ok_or(())
    }
}

pub static TASK_MANAGER: RwLock<TaskManager> = RwLock::new(TaskManager::new());
