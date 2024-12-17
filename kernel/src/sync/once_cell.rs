use core::cell::UnsafeCell;

use spin::Mutex;

pub struct OnceCell<T> {
    inner:       UnsafeCell<Option<T>>,
    initialized: Mutex<bool>,
}

#[allow(dead_code)]
impl<T> OnceCell<T> {
    pub const fn new() -> Self {
        Self {
            inner:       UnsafeCell::new(None),
            initialized: Mutex::new(false),
        }
    }

    pub fn set(&self, value: T) -> Result<(), OnceCellAlreadySetError> {
        let mut initialized = self.initialized.lock();

        if *initialized {
            return Err(OnceCellAlreadySetError);
        }

        unsafe {
            *self.inner.get() = Some(value);
        }
        *initialized = true;

        Ok(())
    }

    pub fn get(&self) -> Option<&T> {
        if *self.initialized.lock() {
            unsafe { (*self.inner.get()).as_ref() }
        } else {
            None
        }
    }

    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        if self.get().is_none() {
            self.set(f()).ok();
        }

        self.get().unwrap()
    }
}

pub struct OnceCellAlreadySetError;

unsafe impl<T> Sync for OnceCell<T> {}
