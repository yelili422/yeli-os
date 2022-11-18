use core::mem;

use alloc::{boxed::Box, vec};

use super::context::ContextSnapshot;

pub struct Proc {
    // pid: u32,
    // state: State,
    stack: Option<Box<[u8]>>,
    context: ContextSnapshot,
    // context: Context,
    // parent: Option<Arc<TaskStruct>>,
    // children: Vec<Arc<TaskStruct>>,
    // exit_code: i32,
}

impl Proc {
    pub fn spawn(func: extern fn()) -> Self {
        Proc {
            stack: {
                let mut stack = vec![0; 65535].into_boxed_slice();
                let offset = stack.len() - mem::size_of::<usize>();
                unsafe {
                    let func_ptr = stack.as_mut_ptr().add(offset);
                    *(func_ptr as *mut usize) = func as usize;
                }
                Some(stack)
            },
            context: ContextSnapshot::empty(),
        }
    }

    pub fn switch_to(&mut self, target: &mut Proc) {
        unsafe {
            // self.context.switch(&mut target.context);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{println, proc::Proc};

    // extern fn spawned_task() {
    //     println!("Spawn new task finished");
    // }

    // #[test_case]
    // fn test_init_task() {
    //     Proc::spawn(spawned_task);
    // }

    // #[no_mangle]
    // extern "C" fn switch(current: &mut Proc, next: &mut Proc) {
    //     println!("thread 1");
    // }

    // #[test_case]
    // fn test_thread_switch() {}
}
