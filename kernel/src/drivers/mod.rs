/// the virtio spec:
/// https://docs.oasis-open.org/virtio/virtio/v1.1/virtio-v1.1.pdf
use bitflags::bitflags;

pub mod virtio;

bitflags! {
    /// device feature bits
    struct VirtIOFeatures: u32 {
        const BLK_F_RO = 1 << 5;	/* Disk is read-only */
        const BLK_F_SCSI = 1 << 7;	/* Supports scsi command passthru */
        const BLK_F_CONFIG_WCE = 1 << 11;	/* Writeback mode available in config */
        const BLK_F_MQ = 1 << 12;	/* support more than one vq */
        const F_ANY_LAYOUT = 1 << 27;
        const RING_F_INDIRECT_DESC = 1 << 28;
        const RING_F_EVENT_IDX = 1 << 29;
    }

    /// status register bits, from qemu virtio_config.h
    struct VirtIOStatus: u32 {
        const ACKNOWLEDGE = 1;
        const DRIVER = 	2;
        const DRIVER_OK = 4;
        const FEATURES_OK = 8;
    }
}

#[repr(transparent)]
pub struct ReadOnly<T: Copy> {
    value: T,
}

impl<T: Copy> ReadOnly<T> {
    pub fn read(&self) -> T {
        unsafe { core::ptr::read_volatile(&self.value) }
    }
}

#[repr(transparent)]
pub struct WriteOnly<T: Copy> {
    value: T,
}

impl<T: Copy> WriteOnly<T> {
    pub fn write(&mut self, value: T) {
        unsafe { core::ptr::write_volatile(&mut self.value, value) }
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct ReadWrite<T: Copy> {
    value: T,
}

impl<T: Copy> ReadWrite<T> {
    pub fn from(value: T) -> Self {
        Self { value }
    }

    pub fn read(&self) -> T {
        unsafe { core::ptr::read_volatile(&self.value) }
    }

    pub fn write(&mut self, value: T) {
        unsafe { core::ptr::write_volatile(&mut self.value, value) }
    }
}

pub type Volatile<T> = ReadWrite<T>;
