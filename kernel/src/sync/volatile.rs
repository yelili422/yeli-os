#[repr(transparent)]
pub struct ReadOnly<T: Copy>(pub T);

impl<T: Copy> ReadOnly<T> {
    pub fn read_volatile(&self) -> T {
        unsafe { core::ptr::read_volatile(&self.0) }
    }
}

#[repr(transparent)]
pub struct WriteOnly<T: Copy>(pub T);

impl<T: Copy> WriteOnly<T> {
    pub fn write_volatile(&mut self, value: T) {
        unsafe { core::ptr::write_volatile(&mut self.0, value) }
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Volatile<T: Copy>(pub T);

impl<T: Copy> Volatile<T> {
    pub fn from(value: T) -> Self {
        Self(value)
    }

    pub fn read_volatile(&self) -> T {
        unsafe { core::ptr::read_volatile(&self.0 as *const _) }
    }

    pub fn write_volatile(&mut self, value: T) {
        unsafe { core::ptr::write_volatile(&mut self.0, value) }
    }
}

pub type ReadWrite<T> = Volatile<T>;
