use alloc::{alloc::alloc_zeroed, boxed::Box, sync::Arc, vec::Vec};
use core::{
    alloc::Layout,
    fmt,
    pin::Pin,
    ptr::copy_nonoverlapping,
    sync::atomic::{AtomicU64, Ordering},
};

use fs::inode::Inode;
use spin::Mutex;

use super::Context;
use crate::{
    fs::ROOT_FS,
    intr::{trampoline, usertrapret, TrapFrame},
    mem::{
        page::{PTEFlags, PageTable},
        KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE, TRAP_FRAME, USER_STACK, USER_STACK_SIZE,
    },
    va2pa,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcId(u64);

#[allow(dead_code)]
impl ProcId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        ProcId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl From<u64> for ProcId {
    fn from(id: u64) -> Self {
        ProcId(id)
    }
}

impl fmt::Display for ProcId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Proc {
    pub pid:          ProcId,
    pub state:        ProcState,
    /// The kernel stack is part of the kernel space. Hence,
    /// it is not directly accessible from a user process.
    pub kernel_stack: Pin<Box<[u8]>>,
    pub user_stack:   Pin<Box<[u8]>>,
    pub context:      Context,
    pub trap_frame:   Pin<Box<TrapFrame>>,
    pub page_table:   Pin<Box<PageTable>>,
    pub working_dir:  Arc<Mutex<Inode>>,
    files:            Vec<Option<Arc<Mutex<Inode>>>>,
}

impl Proc {
    pub fn new(src: &[u8]) -> Self {
        assert_eq!(size_of::<TrapFrame>(), PAGE_SIZE);

        let pid = ProcId::new();
        // ===============================================================
        // FIXME:
        // Allocating the kernel stack in heap.
        // THIS IS A BAD IDEA.
        // We can't recognize the address in the range of the 'process' kernel
        // stack area is safe or not, even if the address is out of it's bounds.
        // User stack is ok, because it is a virtual address mapped in a separate area.
        // When sp is out of the bounds, a page fault will occur.
        // ===============================================================
        let kernel_stack = Box::pin([0u8; KERNEL_STACK_SIZE]);
        let user_stack = Box::pin([0u8; USER_STACK_SIZE]);

        let mut trap_frame = unsafe { Box::<TrapFrame>::new_zeroed().assume_init() };
        // Prepare for the very first "return" form kernel to user.
        trap_frame.epc = 0; // user program counter
        trap_frame.sp = USER_STACK; // user stack pointer

        let mut context = Context::default();
        // Set up new context to start executing at `usertrapret`,
        // which returns to user space. Since, we set `sp` to kernel
        // stack temporarily.
        context.ra = usertrapret as usize;
        context.sp = kernel_stack.as_ptr() as usize + kernel_stack.len();

        let root = ROOT_FS.get().unwrap().root();

        let page_table = unsafe { Box::<PageTable>::new_zeroed().assume_init() };
        let mut proc = Proc {
            pid,
            state: ProcState::Init,
            kernel_stack,
            user_stack,
            context,
            trap_frame: Box::into_pin(trap_frame),
            page_table: Box::into_pin(page_table),
            working_dir: root,
            files: Vec::with_capacity(16),
        };
        proc.uvm_init(src);

        proc
    }

    fn uvm_init(&mut self, src: &[u8]) {
        let mut page_table = self.page_table.as_mut();
        let trap_frame = self.trap_frame.as_ref();
        let user_stack = self.user_stack.as_ref();
        unsafe {
            map_user_stack(&mut page_table, va2pa!(&user_stack.as_ptr() as *const _ as usize));
            map_init_code(&mut page_table, src);

            // for user trap
            map_trampoline(&mut page_table);
            map_trap_frame(&mut page_table, va2pa!(&*trap_frame as *const _ as usize));
        }
    }

    // pub fn open_file(&mut self, inode: Arc<Mutex<Inode>>) -> Result<usize, OpenFileError> {
    //     for (fd, file) in self.files.iter_mut().enumerate() {
    //         if file.is_none() {
    //             *file = Some(inode);
    //             return Ok(fd);
    //         }
    //     }
    //     Err(OpenFileError::NoMoreSpace)
    // }

    pub fn close_file(&mut self, fd: usize) {
        if fd > self.files.len() {
            return;
        }
        self.files[fd] = None;
    }

    pub fn close_all_files(&mut self) {
        for fd in 0..self.files.len() {
            self.close_file(fd);
        }
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        if self.state != ProcState::Zombie {
            panic!("unexpected drop of process, pid: {}", self.pid);
        }
        self.close_all_files();
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum OpenFileError {
    NoMoreSpace,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ProcState {
    Init,
    Runnable,
    Running,
    Zombie,
    Exited(i32),
    Sleeping,
}

// Map trampoline code (for system call return) at the hightest
// user virtual address. Only the supervisor uses it, on the
// way to/from user space, so not PTE::U.
unsafe fn map_trampoline(pt: &mut PageTable) {
    pt.map(
        TRAMPOLINE,
        va2pa!(trampoline as usize),
        PAGE_SIZE,
        PTEFlags::R | PTEFlags::X | PTEFlags::G,
    );
}

// Map the trap frame just below TRAMPOLINE,
// for the trampoline.S.
unsafe fn map_trap_frame(pt: &mut PageTable, trap_frame_pa: usize) {
    pt.map(TRAP_FRAME, trap_frame_pa, PAGE_SIZE, PTEFlags::R | PTEFlags::W);
}

unsafe fn map_init_code(pt: &mut PageTable, src: &[u8]) {
    assert!(src.len() <= 2 * PAGE_SIZE, "user init data too large");

    let layout = Layout::from_size_align(PAGE_SIZE * 2, PAGE_SIZE).unwrap();
    let page = unsafe { alloc_zeroed(layout) };
    unsafe { copy_nonoverlapping(src.as_ptr(), page, layout.size()) };

    pt.map(
        0,
        va2pa!(page as *mut u8 as usize),
        layout.size(),
        PTEFlags::R | PTEFlags::W | PTEFlags::X | PTEFlags::U,
    );
}

unsafe fn map_user_stack(pt: &mut PageTable, user_stack_pa: usize) {
    pt.map(
        USER_STACK,
        user_stack_pa,
        USER_STACK_SIZE,
        PTEFlags::R | PTEFlags::W | PTEFlags::U,
    );
}
