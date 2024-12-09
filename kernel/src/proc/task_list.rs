use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec};

use log::{debug, info};
use spin::RwLock;

use super::{State, Task, TaskId, MAX_PROC};
use crate::{
    intr::{usertrapret, TrapFrame},
    proc::{Context, KERNEL_STACK_SIZE},
};

// a user program that calls exec("/init")
// assembled from ../user/initcode.S
// od -t xC ../user/initcode
#[rustfmt::skip]
static INITCODE: [u8; 52] = [
    0x17, 0x05, 0x00, 0x00, 0x13, 0x05, 0x45, 0x02,
    0x97, 0x05, 0x00, 0x00, 0x93, 0x85, 0x35, 0x02,
    0x93, 0x08, 0x70, 0x00, 0x73, 0x00, 0x00, 0x00,
    0x93, 0x08, 0x20, 0x00, 0x73, 0x00, 0x00, 0x00,
    0xef, 0xf0, 0x9f, 0xff, 0x2f, 0x69, 0x6e, 0x69,
    0x74, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00
];

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

        let kernel_stack = Box::pin([0u8; KERNEL_STACK_SIZE]);
        let mut trap_frame = TrapFrame::default();
        // Prepare for the very first "return" form kernel to user.
        trap_frame.epc = 0; // user program counter
        trap_frame.sp = kernel_stack.len(); // user stack pointer

        let mut context = Context::default();
        // Set up new context to start executing at `usertrapret`,
        // which returns to user space. Since, we set `sp` to kernel
        // stack temporarily.
        context.ra = usertrapret as usize;
        context.sp = kernel_stack.as_ptr() as usize + kernel_stack.len();

        let task = Task {
            pid,
            state: State::Init,
            kernel_stack,
            context,
            trap_frame,
            page_table: None,
        };

        assert!(self
            .tasks
            .insert(pid, Arc::new(RwLock::new(task)))
            .is_none());
        debug!("proc: allocated new task: {}", pid);

        Ok(self.tasks.get(&pid).unwrap())
    }

    pub fn current(&self) -> Result<&Arc<RwLock<Task>>, ()> {
        // TODO:
        self.tasks.get(&0).ok_or(())
    }

    pub fn user_init(&mut self) {
        info!("Initializing the init userspace...");

        let task_lock = self.new_task().expect("failed to create init task");
        {
            let mut task = task_lock.write();
            assert_eq!(task.pid, 0, "The first pid is not 0");

            task.init_user_page_table();
            task.page_table
                .as_mut()
                .unwrap()
                .as_mut()
                .user_vm_init(&INITCODE);

            task.state = State::Runnable;
        }
    }
}
