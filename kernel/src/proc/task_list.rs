use alloc::{collections::BTreeMap, sync::Arc, vec};
use spin::RwLock;

use crate::{intr::usertrapret, mem::page::PageTable};

use super::{State, Task, TaskId, KERNEL_STACK_SIZE, MAX_PROC};

pub struct TaskList {
    tasks:   BTreeMap<TaskId, Arc<RwLock<Task>>>,
    next_id: u64,
}

impl TaskList {
    pub const fn new() -> Self {
        TaskList {
            tasks:   BTreeMap::new(),
            next_id: 0,
        }
    }

    pub fn get(&self, id: &TaskId) -> Option<&Arc<RwLock<Task>>> {
        self.tasks.get(id)
    }

    pub fn alloc_pid(&mut self) -> TaskId {
        self.next_id += 1;
        self.next_id - 1
    }

    pub fn new_task(&mut self) -> Result<&Arc<RwLock<Task>>, ()> {
        let pid = self.alloc_pid();
        if pid > MAX_PROC {
            panic!("too many processes.")
        }

        let task = Task::new(pid);
        assert!(self
            .tasks
            .insert(pid, Arc::new(RwLock::new(task)))
            .is_none());

        Ok(self.tasks.get(&pid).unwrap())
    }

    pub fn spawn(&mut self, _func: extern "C" fn()) -> Result<&Arc<RwLock<Task>>, ()> {
        let task_lock = self.new_task()?;
        {
            let mut task = task_lock.write();

            let stack = vec![0; KERNEL_STACK_SIZE].into_boxed_slice();

            // Prepare for the very first "return" form kernel to user.
            task.trap_frame.epc = 0; // user program counter
            task.trap_frame.sp = stack.len(); // user stack pointer

            // Set up new context to start executing at `usertrapret`,
            // which returns to user space. Since, we set `sp` to kernel
            // stack temporarily.
            task.context.ra = usertrapret as usize;
            task.context.sp = stack.as_ptr() as usize + stack.len();

            task.kernel_stack = Some(stack);
            task.set_user_page_table(PageTable::empty());
        }
        Ok(task_lock)
    }

    pub fn current(&self) -> Result<&Arc<RwLock<Task>>, ()> {
        // TODO:
        self.tasks.get(&0).ok_or(())
    }

    pub fn user_init(&mut self) {
        match self.spawn(userspace_init) {
            Ok(init_task_lock) => {
                let mut init_task = init_task_lock.write();
                assert_eq!(init_task.pid, 0, "The first pid is not 0");

                init_task.state = State::Runnable;
            }
            Err(_) => {
                panic!("failed to init userspace.");
            }
        }
    }
}

pub extern "C" fn userspace_init() {
    panic!("userspace_init")
}
