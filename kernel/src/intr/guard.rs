use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use log::{debug, trace};
use riscv::register::sstatus;

pub struct InterruptManager {
    noff:   AtomicU32,
    intena: AtomicBool,
}

impl InterruptManager {
    pub const fn new() -> Self {
        Self {
            noff:   AtomicU32::new(0),
            intena: AtomicBool::new(false),
        }
    }

    fn push_off(&self) {
        unsafe {
            let old_enable = sstatus::read().sie();
            sstatus::clear_sie();
            if self.noff.load(Ordering::SeqCst) == 0 {
                self.intena.store(old_enable, Ordering::SeqCst);
            }
        }
        self.noff.fetch_add(1, Ordering::SeqCst);
        trace!(
            "intr: push_off: {}, intena: {}",
            self.noff.load(Ordering::SeqCst),
            self.intena.load(Ordering::SeqCst),
        );
    }

    fn pop_off(&self) {
        trace!(
            "intr: pop_off: {}, intena: {}",
            self.noff.load(Ordering::SeqCst),
            self.intena.load(Ordering::SeqCst)
        );
        if sstatus::read().sie() {
            panic!("pop_off(): interruptable");
        }
        let noff = self.noff.fetch_sub(1, Ordering::SeqCst);
        if noff == 1 && self.intena.load(Ordering::SeqCst) {
            debug!("intr: pop_off: enable interrupt");
            unsafe { sstatus::set_sie() };
        } else if noff == 0 {
            panic!("pop_off(): noff underflow");
        }
    }

    pub fn enter_critical<'a>(&'a mut self) -> InterruptGuard<'a> {
        self.push_off();
        InterruptGuard { manager: self }
    }
}

pub struct InterruptGuard<'a> {
    manager: &'a mut InterruptManager,
}

impl<'a> Drop for InterruptGuard<'a> {
    fn drop(&mut self) {
        self.manager.pop_off();
    }
}
