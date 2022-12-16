use core::any::Any;

use alloc::{collections::BTreeMap, sync::Arc};
use log::info;
use spin::RwLock;

use crate::{
    println,
    proc::{switch_to, Context, ContextId, Proc, State, MAX_PROC},
};

pub struct TaskManager {
    inner: RwLock<TaskManagerInner>,
}

impl TaskManager {
    pub const fn new() -> Self {
        TaskManager {
            inner: RwLock::new(TaskManagerInner {
                tasks: BTreeMap::new(),
                next_id: 0,
            }),
        }
    }

    pub fn user_init(&self) {
        let init_proc_context: *const Context;

        let mut inner = self.inner.write();
        {
            let pid = 0;
            let proc = Arc::new(RwLock::new(Proc::from_fn(pid, init_proc)));

            inner.tasks.insert(pid, proc);

            let init_proc = inner.tasks.get(&pid).unwrap();
            let init_proc_inner = init_proc.read();
            {
                init_proc_context = &init_proc_inner.context;
            }
        }

        unsafe { switch_to(&mut Context::default(), init_proc_context) }

        panic!("unreachable.");
    }

    pub fn spawn(&self, _func: extern "C" fn()) -> Result<&Arc<RwLock<Proc>>, ()> {
        let inner = self.inner.write();
        {
            if inner.next_id > MAX_PROC {
                unimplemented!();
            }
        }
        todo!()
    }

    pub fn schedule(&self) -> ! {
        loop {
            let inner = self.inner.write();
            for (_, proc) in inner.tasks.iter() {
                let mut proc = proc.write();
                if proc.state == State::Runnable {
                    proc.state = State::Running;

                    unsafe {
                        switch_to(&mut Context::default(), &mut proc.context);
                    }
                }
            }
        }
    }
}

extern "C" fn init_proc() {
    info!("init_proc is running.")
}

pub struct TaskManagerInner {
    tasks: BTreeMap<ContextId, Arc<RwLock<Proc>>>,
    next_id: u32,
}

pub static TASK_MANAGER: TaskManager = TaskManager::new();
