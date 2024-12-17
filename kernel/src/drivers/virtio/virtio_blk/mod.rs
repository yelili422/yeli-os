use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::{Arc, Weak},
};
use core::array::from_fn;

use fs::block_dev::{BlockDevice, BLOCK_SIZE};
use log::{debug, info, trace};
use spin::Mutex;

use super::{VirtIOError, VirtIOInitError, VirtIORegs, VirtQueue, VirtqDesc, VirtqDescFlags};
use crate::{
    drivers::{
        virtio::{VirtIODeviceType, VirtIOFeatures, VirtIOStatus, CONFIG_SPACE_OFFSET, QUEUE_SIZE},
        Volatile,
    },
    va2pa,
};

const MAX_BLK_DEVICES: usize = 16;

#[derive(Clone, Copy, Debug)]
enum VirtIOBlockReqType {
    Read  = 0,
    Write = 1,
}

/// Virtio block device configuration.
/// see spec.5.2.4
#[repr(C)]
pub struct VirtIOBlockConfig {
    pub capacity:                 u64,                 // le64
    pub size_max:                 u32,                 // le32
    pub seg_max:                  u32,                 // le32
    pub geometry:                 VirtIOBlockGeometry, // struct
    pub blk_size:                 u32,                 // le32
    pub topology:                 VirtIOBlockTopology, // struct
    pub writeback:                u8,                  // u8
    pub unused0:                  [u8; 3],             // padding to align the next field
    pub max_discard_sectors:      u32,                 // le32
    pub max_discard_seg:          u32,                 // le32
    pub discard_sector_alignment: u32,                 // le32
    pub max_write_zeroes_sectors: u32,                 // le32
    pub max_write_zeroes_seg:     u32,                 // le32
    pub write_zeroes_may_unmap:   u8,                  // u8
    pub unused1:                  [u8; 3],             // padding
}

#[repr(C)]
pub struct VirtIOBlockGeometry {
    pub cylinders: u16, // le16
    pub heads:     u8,  // u8
    pub sectors:   u8,  // u8
}

#[repr(C)]
pub struct VirtIOBlockTopology {
    pub physical_block_exp: u8,  // u8
    pub alignment_offset:   u8,  // u8
    pub min_io_size:        u16, // le16
    pub opt_io_size:        u32, // le32
}

/// the format of the first descriptor in a disk request.
/// to be followed by two more descriptors containing
/// the block, and a one-byte status.
#[repr(C)]
struct VirtIOBlockReq {
    type_:    u32,
    reserved: u32,
    sector:   u64,
}

struct InnerVirtIOBlock {
    regs:        *mut VirtIORegs,
    queue:       Box<VirtQueue>,
    used_idx:    u16,
    sectors_num: u64,
    status:      [Volatile<VirtIORequestStatus>; QUEUE_SIZE],
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum VirtIORequestStatus {
    Pending = 0,
    Done    = 1,
}

pub struct VirtIOBlock {
    inner:    Mutex<InnerVirtIOBlock>,
    capacity: u64, // bytes
}

impl VirtIOBlock {
    pub fn init(header: usize) -> Result<Arc<Self>, VirtIOInitError> {
        let regs = unsafe { &mut *(header as *mut VirtIORegs) };

        if regs.magic.read_volatile() != 0x74726976 {
            return Err(VirtIOInitError::InvalidMagic(regs.magic.read_volatile()));
        }

        if regs.version.read_volatile() != VirtIODeviceType::BlockDevice as u32 {
            return Err(VirtIOInitError::InvalidVersion(regs.version.read_volatile()));
        }

        let block_config =
            unsafe { &*((header + CONFIG_SPACE_OFFSET) as *const VirtIOBlockConfig) };
        info!("Device capacity: {} sectors", block_config.capacity);

        regs.status.write_volatile(VirtIOStatus::empty().bits());
        regs.status.write_volatile(VirtIOStatus::ACKNOWLEDGE.bits());
        regs.status.write_volatile(VirtIOStatus::DRIVER.bits());

        // negotiate features
        let mut features = VirtIOFeatures::from_bits_truncate(regs.device_features.read_volatile());
        features.remove(
            VirtIOFeatures::BLK_F_RO
                | VirtIOFeatures::BLK_F_SCSI
                | VirtIOFeatures::BLK_F_CONFIG_WCE
                | VirtIOFeatures::BLK_F_MQ
                | VirtIOFeatures::F_ANY_LAYOUT
                | VirtIOFeatures::RING_F_EVENT_IDX
                | VirtIOFeatures::RING_F_INDIRECT_DESC,
        );
        regs.driver_features.write_volatile(features.bits());
        regs.status.write_volatile(VirtIOStatus::FEATURES_OK.bits());

        let queue = Box::new(VirtQueue::new());
        regs.queue_sel.write_volatile(0);
        assert_eq!(regs.queue_ready.read_volatile(), 0, "virtio disk should not be ready");

        regs.queue_num.write_volatile(va2pa!(QUEUE_SIZE as u32));
        regs.queue_desc_low
            .write_volatile(va2pa!(queue.desc.as_ptr() as u32));
        regs.queue_desc_high
            .write_volatile(va2pa!(((queue.desc.as_ptr() as u64) >> 32) as u32));
        regs.queue_driver_low
            .write_volatile(va2pa!(queue.avail.as_ptr() as u32));
        regs.queue_driver_high
            .write_volatile(va2pa!(((queue.avail.as_ptr() as u64) >> 32) as u32));
        regs.queue_device_low
            .write_volatile(va2pa!(queue.used.as_ptr() as u32));
        regs.queue_device_high
            .write_volatile(va2pa!(((queue.used.as_ptr() as u64) >> 32) as u32));

        regs.queue_ready.write_volatile(1);
        regs.status.write_volatile(VirtIOStatus::DRIVER_OK.bits());

        let block = Arc::new(VirtIOBlock {
            inner:    Mutex::new(InnerVirtIOBlock {
                regs,
                queue,
                used_idx: 0,
                sectors_num: block_config.capacity,
                status: from_fn(|_| Volatile::from(VirtIORequestStatus::Pending)),
            }),
            capacity: block_config.capacity * 512,
        });

        // SAFETY: We only register device at this os startup.
        unsafe { VIRTIO_BLK_DEVICES[0] = Some(Arc::downgrade(&block)) };
        Ok(block)
    }

    pub fn read_block(&self, block_id: u64, buf: &mut [u8]) -> Result<(), VirtIOError> {
        if buf.len() != BLOCK_SIZE {
            return Err(VirtIOError::InvalidBufferSize(buf.len()));
        }
        self.send(block_id, buf.as_ptr(), VirtIOBlockReqType::Read)
    }

    pub fn write_block(&self, block_id: u64, buf: &[u8]) -> Result<(), VirtIOError> {
        if buf.len() != BLOCK_SIZE {
            return Err(VirtIOError::InvalidBufferSize(buf.len()));
        }
        self.send(block_id, buf.as_ptr(), VirtIOBlockReqType::Write)
    }

    fn send(
        &self,
        block_id: u64,
        buf_ptr: *const u8,
        op: VirtIOBlockReqType,
    ) -> Result<(), VirtIOError> {
        assert_eq!(BLOCK_SIZE % 512, 0);

        let mut inner = self.inner.lock();
        {
            let sector = block_id * (BLOCK_SIZE as u64 / 512);
            let sector_end = sector + (BLOCK_SIZE as u64 / 512);
            if sector_end >= inner.sectors_num {
                return Err(VirtIOError::OutOfCapacity(sector_end));
            };

            trace!("virtio: reading/writing block: {}, sector: {}, op: {:?}", block_id, sector, op);

            // build request header
            let header = Box::new(VirtIOBlockReq {
                type_:    op as u32,
                reserved: 0,
                sector:   sector as u64,
            });

            let status: Box<u8> = Box::new(0xff); // device writes 0 on success
            let status_ptr = &*status as *const u8;

            let desc = unsafe { inner.queue.desc.as_mut() };
            desc[0] = VirtqDesc {
                addr:  va2pa!(&*header as *const _ as u64),
                len:   core::mem::size_of::<VirtIOBlockReq>() as u32,
                flags: VirtqDescFlags::NEXT.bits(),
                next:  1,
            };

            desc[1] = VirtqDesc {
                addr:  va2pa!(buf_ptr as u64),
                len:   BLOCK_SIZE as u32,
                flags: match op {
                    VirtIOBlockReqType::Read => {
                        (VirtqDescFlags::NEXT | VirtqDescFlags::WRITE).bits()
                    }
                    VirtIOBlockReqType::Write => VirtqDescFlags::NEXT.bits(),
                },
                next:  2,
            };

            desc[2] = VirtqDesc {
                addr:  va2pa!(status_ptr as u64),
                len:   1,
                flags: VirtqDescFlags::WRITE.bits(),
                next:  0,
            };

            // notify device
            let avail = unsafe { inner.queue.avail.as_mut() };

            let avail_idx = avail.idx.read_volatile();
            avail.ring[avail_idx as usize % QUEUE_SIZE] = Volatile::from(0);
            avail.idx.write_volatile(avail_idx + 1);

            unsafe {
                (*inner.regs).queue_notify.write_volatile(0);
            }

            // TODO: move to interrupt handler
            // wait device
            loop {
                let used = unsafe { inner.queue.used.read_volatile() };
                if used.idx.read_volatile() != inner.used_idx {
                    let id = used.ring[inner.used_idx as usize % QUEUE_SIZE]
                        .id
                        .read_volatile();
                    trace!("virtio: finished operation id: {}", id);
                    break;
                }
            }
            inner.used_idx = inner.used_idx.wrapping_add(1);
            assert_eq!(unsafe { status_ptr.read_volatile() }, 0);

            // TODO: change loop to sleep
            // inner.status[0] = Volatile::from(VirtIORequestStatus::Pending);
            // while inner.status[0].read_volatile() == VirtIORequestStatus::Pending {}
        }
        Ok(())
    }

    pub fn handle_interrupt(&self) {
        debug!("virtio: handling interrupt");
        let mut inner = self.inner.lock();
        {
            let used = unsafe { inner.queue.used.read_volatile() };
            while inner.used_idx != used.idx.read_volatile() {
                let queue_used = unsafe { inner.queue.used.read() };
                let id = queue_used.ring[inner.used_idx as usize % QUEUE_SIZE]
                    .id
                    .read_volatile();
                trace!("virtio: finished operation id: {}", id);

                inner.status[id as usize] = Volatile::from(VirtIORequestStatus::Done);
                inner.used_idx = inner.used_idx.wrapping_add(1);
            }
        }
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}

impl Drop for VirtIOBlock {
    fn drop(&mut self) {
        debug!("virtio: dropping block device");
        unsafe { VIRTIO_BLK_DEVICES[0] = None };
    }
}

unsafe impl Sync for VirtIOBlock {}
unsafe impl Send for VirtIOBlock {}

pub static mut VIRTIO_BLK_DEVICES: [Option<Weak<VirtIOBlock>>; MAX_BLK_DEVICES] =
    [const { None }; MAX_BLK_DEVICES];

impl BlockDevice for VirtIOBlock {
    fn read(&self, block_id: u64, buf: &mut [u8]) -> Result<(), String> {
        self.read_block(block_id, buf)
            .map_err(|err| err.to_string())
    }

    fn write(&self, block_id: u64, buf: &[u8]) -> Result<(), String> {
        self.write_block(block_id, buf)
            .map_err(|err| err.to_string())
    }
}
