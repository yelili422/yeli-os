pub mod virtio_blk;

use alloc::boxed::Box;
use core::ptr::NonNull;

use bitflags::bitflags;
use virtio_blk::VIRTIO_BLK_DEVICES;

use super::{ReadOnly, ReadWrite, Volatile, WriteOnly};

/// Virtqueue size.
const QUEUE_SIZE: usize = 16;

/// Device-specific configuration space starts at the offset 0x100 and is ac-
/// cessed with byte alignment. Its meaning and size depend on the device
/// and the driver.
///
/// Same as [VirtIORegs::config]
const CONFIG_SPACE_OFFSET: usize = 0x100;

/// Virtio device type
///
/// see spec.5
#[allow(unused)]
#[derive(Debug)]
pub enum VirtIODeviceType {
    Reserved         = 0,            // Reserved (invalid)
    NetworkCard      = 1,            // Network card
    BlockDevice      = 2,            // Block device
    Console          = 3,            // Console
    EntropySource    = 4,            // Entropy source
    MemoryBallooningTraditional = 5, // Memory ballooning (traditional)
    IoMemory         = 6,            // I/O Memory
    Rpmsg            = 7,            // RPMsg
    ScsiHost         = 8,            // SCSI host
    Transport9P      = 9,            // 9P transport
    Mac80211Wlan     = 10,           // mac80211 WLAN
    RprocSerial      = 11,           // Remote processor serial
    VirtioCaif       = 12,           // Virtio CAIF
    MemoryBalloon    = 13,           // Memory balloon
    GpuDevice        = 16,           // GPU device
    TimerClockDevice = 17,           // Timer/Clock device
    InputDevice      = 18,           // Input device
    SocketDevice     = 19,           // Socket device
    CryptoDevice     = 20,           // Crypto device
    SignalDistributionModule = 21,   // Signal Distribution Module
    PstoreDevice     = 22,           // Persistent storage device
    IommuDevice      = 23,           // IOMMU device
    MemoryDevice     = 24,           // Memory device
}

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

    struct VirtqDescFlags: u16 {
        const NEXT = 1;
        const WRITE = 2;
    }
}

#[rustfmt::skip]
#[allow(unused)]
#[repr(C)]
pub struct VirtIORegs {
    /* 0x000 */ magic:               ReadOnly<u32>, // Magic value 0x74726976 ("virt")
    /* 0x004 */ version:             ReadOnly<u32>, // Device version number, 0x2 (0x1 for legacy devices)
    /* 0x008 */ device_id:           ReadOnly<u32>, // Virtio Subsystem Device ID
    /* 0x00c */ vendor_id:           ReadOnly<u32>, // Virtio Subsystem Vendor ID
    /* 0x010 */ device_features:     ReadOnly<u32>, // Flags representing features the device supports
    /* 0x014 */ device_features_sel: WriteOnly<u32>, // Device (host) features word selection

    // 0x018 - 0x01c: Reserved (padding to align the next field to 0x020)
    _reserved1:         [u8; 8],

    /* 0x020 */ driver_features:     WriteOnly<u32>, // Flags representing features understood and activated by the driver
    /* 0x024 */ driver_features_sel: WriteOnly<u32>, // Activated (guest) features word selection

    // 0x028 - 0x02c: Reserved (padding to align the next field to 0x030)
    _reserved2:         [u8; 8],

    /* 0x030 */ queue_sel:           WriteOnly<u32>, // Virtual queue index
    /* 0x034 */ queue_num_max:       ReadOnly<u32>, // Maximum virtual queue size
    /* 0x038 */ queue_num:           WriteOnly<u32>, // Virtual queue size

    // 0x03c - 0x040: Reserved (padding to align the next field to 0x044)
    _reserved3:         [u8; 8],

    /* 0x044 */ queue_ready:         ReadWrite<u32>, // Virtual queue ready bit

    // 0x048 - 0x04c: Reserved (padding to align the next field to 0x050)
    _reserved4:         [u8; 8],

    /* 0x050 */ queue_notify:        WriteOnly<u32>, // Queue notifier

    // 0x054 - 0x05c: Reserved (padding to align the next field to 0x060)
    _reserved5:         [u8; 12],

    /* 0x060 */ interrupt_status:    ReadOnly<u32>, // Interrupt status
    /* 0x064 */ interrupt_ack:       WriteOnly<u32>, // Interrupt acknowledge

    // 0x068 - 0x06c: Reserved (padding to align the next field to 0x070)
    _reserved6:         [u8; 8],

    /* 0x070 */ status:              ReadWrite<u32>, // Device status

    // 0x074 - 0x07c: Reserved (padding to align the next field to 0x080)
    _reserved7:         [u8; 12],

    /* 0x080 */ queue_desc_low:      WriteOnly<u32>, // Virtual queue’s Descriptor Area (low 32 bits)
    /* 0x084 */ queue_desc_high:     WriteOnly<u32>, // Virtual queue’s Descriptor Area (high 32 bits)

    // 0x088 - 0x08c: Reserved (padding to align the next field to 0x090)
    _reserved8:         [u8; 8],
    /* 0x090 */ queue_driver_low:    WriteOnly<u32>, // Virtual queue’s Driver Area (low 32 bits)
    /* 0x094 */ queue_driver_high:   WriteOnly<u32>, // Virtual queue’s Driver Area (high 32 bits)

    // 0x098 - 0x09c: Reserved (padding to align the next field to 0x0a0)
    _reserved9:         [u8; 8],

    /* 0x0a0 */ queue_device_low:    WriteOnly<u32>, // Virtual queue’s Device Area (low 32 bits)
    /* 0x0a4 */ queue_device_high:   WriteOnly<u32>, // Virtual queue’s Device Area (high 32 bits)

    // 0x0a8 - 0x0fc: Reserved (padding to align the next field to 0x0fc)
    _reserved10:        [u8; 84],

    /* 0x0fc */ config_generation:   ReadOnly<u32>, // Configuration atomicity value

    // 0x100+: Device-specific configuration space
    config:             [u8; 0],      // Configuration space placeholder
}

struct VirtQueue {
    desc:  NonNull<[VirtqDesc; QUEUE_SIZE]>,
    avail: NonNull<VirtqAvail>,
    used:  NonNull<VirtqUsed>,
}

impl VirtQueue {
    pub fn new() -> Self {
        let desc = Box::new(core::array::from_fn(|_| VirtqDesc {
            addr:  0,
            len:   0,
            flags: 0,
            next:  0,
        }));
        let desc_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(desc)) };

        let avail = Box::new(VirtqAvail {
            flags:      Volatile::from(0),
            idx:        Volatile::from(0),
            ring:       core::array::from_fn(|_| Volatile::from(0)),
            used_event: Volatile::from(0),
        });
        let avail_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(avail)) };

        let used = Box::new(VirtqUsed {
            flags:       Volatile::from(0),
            idx:         Volatile::from(0),
            ring:        core::array::from_fn(|_| VirtqUsedElem {
                id:  Volatile::from(0),
                len: Volatile::from(0),
            }),
            avail_event: Volatile::from(0),
        });
        let used_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(used)) };

        Self {
            desc:  desc_ptr,
            avail: avail_ptr,
            used:  used_ptr,
        }
    }
}

#[repr(C, align(16))]
struct VirtqDesc {
    addr:  u64,
    len:   u32,
    flags: u16,
    next:  u16,
}

#[repr(C, align(2))]
struct VirtqAvail {
    flags:      Volatile<u16>,
    idx:        Volatile<u16>,
    ring:       [Volatile<u16>; QUEUE_SIZE],
    used_event: Volatile<u16>, /* Only if VIRTIO_F_EVENT_IDX */
}

#[repr(C, align(4))]
struct VirtqUsed {
    flags:       Volatile<u16>,
    idx:         Volatile<u16>,
    ring:        [VirtqUsedElem; QUEUE_SIZE],
    avail_event: Volatile<u16>, /* Only if VIRTIO_F_EVENT_IDX */
}

#[repr(C)]
struct VirtqUsedElem {
    id:  Volatile<u32>, // first descriptor index of chain
    len: Volatile<u32>, // wrote bytes
}

#[derive(Debug)]
pub enum VirtIOInitError {
    /// Invalid magic number 0x74726976
    InvalidMagic(u32),

    /// Invalid or unsupported virtio version.
    InvalidVersion(u32),
}

#[derive(Debug)]
pub enum VirtIOError {
    /// Buffer size must be BLOCK_SIZE bytes.
    InvalidBufferSize(usize),

    /// Read/Write request beyond capacity.
    OutOfCapacity(u64),
}

impl core::fmt::Display for VirtIOError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VirtIOError::InvalidBufferSize(len) => write!(f, "Invalid buffer size: {}", len),
            VirtIOError::OutOfCapacity(sector) => write!(f, "Out of capacity: {}", sector),
        }
    }
}

pub fn handle_virtio_interrupt() {
    // SAFETY: interrupt handler guarantee that only one thread running this
    // function at the same time
    #[allow(static_mut_refs)]
    for device in unsafe { VIRTIO_BLK_DEVICES.iter_mut() } {
        if let Some(block_dev) = device {
            if let Some(block_dev) = block_dev.upgrade() {
                block_dev.handle_interrupt();
            }
        }
    }
}
