/// the virtio spec:
/// https://docs.oasis-open.org/virtio/virtio/v1.1/virtio-v1.1.pdf
pub mod virtio;

#[repr(transparent)]
pub struct ReadOnly<T: Copy> {
    value: T,
}

impl<T: Copy> ReadOnly<T> {
    pub fn read_volatile(&self) -> T {
        unsafe { core::ptr::read_volatile(&self.value) }
    }
}

#[repr(transparent)]
pub struct WriteOnly<T: Copy> {
    value: T,
}

impl<T: Copy> WriteOnly<T> {
    pub fn write_volatile(&mut self, value: T) {
        unsafe { core::ptr::write_volatile(&mut self.value, value) }
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Volatile<T: Copy> {
    value: T,
}

impl<T: Copy> Volatile<T> {
    pub fn from(value: T) -> Self {
        Self { value }
    }

    pub fn read_volatile(&self) -> T {
        unsafe { core::ptr::read_volatile(&self.value) }
    }

    pub fn write_volatile(&mut self, value: T) {
        unsafe { core::ptr::write_volatile(&mut self.value, value) }
    }
}

pub type ReadWrite<T> = Volatile<T>;
