use alloc::boxed::Box;
use core::sync::atomic::Ordering;

use fs::block_dev::BLOCK_SIZE;
use log::{debug, info};

use super::{VirtIOBlockReqType, VirtIORegs, VirtQueue, VirtqDesc, VirtqDescFlags};
use crate::drivers::{
    virtio::{CONFIG_SPACE_OFFSET, QUEUE_SIZE},
    VirtIOFeatures, VirtIOStatus, Volatile,
};

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

pub struct VirtIOBlock {
    base:     *mut VirtIORegs,
    queue:    Box<VirtQueue>,
    used_idx: u16,
    capacity: u64, // sectors
}

impl VirtIOBlock {
    pub fn new(header: usize) -> Result<Self, &'static str> {
        let regs = unsafe { &mut *(header as *mut VirtIORegs) };

        if regs.magic.read() != 0x74726976 {
            return Err("Invalid magic number");
        }

        if regs.version.read() != 2 {
            return Err("Invalid version");
        }

        let block_config =
            unsafe { &*((header + CONFIG_SPACE_OFFSET) as *const VirtIOBlockConfig) };
        info!("Device capacity: {} sectors", block_config.capacity);

        regs.status.write(VirtIOStatus::empty().bits());
        regs.status.write(VirtIOStatus::ACKNOWLEDGE.bits());
        regs.status.write(VirtIOStatus::DRIVER.bits());

        // negotiate features
        let mut features = VirtIOFeatures::from_bits_truncate(regs.device_features.read());
        features.remove(
            VirtIOFeatures::BLK_F_RO
                | VirtIOFeatures::BLK_F_SCSI
                | VirtIOFeatures::BLK_F_CONFIG_WCE
                | VirtIOFeatures::BLK_F_MQ
                | VirtIOFeatures::F_ANY_LAYOUT
                | VirtIOFeatures::RING_F_EVENT_IDX
                | VirtIOFeatures::RING_F_INDIRECT_DESC,
        );
        regs.driver_features.write(features.bits());
        regs.status.write(VirtIOStatus::FEATURES_OK.bits());

        let queue = Box::new(VirtQueue::new());
        regs.queue_sel.write(0);
        assert_eq!(regs.queue_ready.read(), 0, "virtio disk should not be ready");
        regs.queue_num.write(QUEUE_SIZE as u32);
        regs.queue_desc_low.write(queue.desc.as_ptr() as u32);
        regs.queue_desc_high
            .write(((queue.desc.as_ptr() as u64) >> 32) as u32);
        regs.queue_driver_low.write(queue.avail.as_ptr() as u32);
        regs.queue_driver_high
            .write(((queue.avail.as_ptr() as u64) >> 32) as u32);
        regs.queue_device_low.write(queue.used.as_ptr() as u32);
        regs.queue_device_high
            .write(((queue.used.as_ptr() as u64) >> 32) as u32);

        regs.queue_ready.write(1);
        regs.status.write(VirtIOStatus::DRIVER_OK.bits());

        Ok(VirtIOBlock {
            base: regs,
            queue,
            used_idx: 0,
            capacity: block_config.capacity,
        })
    }

    pub fn read_block(&mut self, block_id: u64, buf: &mut [u8]) -> Result<(), &'static str> {
        assert_eq!(BLOCK_SIZE % 512, 0);

        if buf.len() != BLOCK_SIZE {
            return Err("Buffer size must be BLOCK_SIZE bytes.");
        }

        let sector = block_id * (BLOCK_SIZE as u64 / 512);
        if sector + (BLOCK_SIZE as u64 / 512) >= self.capacity {
            return Err("read/write request beyond capacity.");
        };

        debug!("[virtio] reading/writing block: {}, sector: {}", block_id, sector);

        // build request header
        let header = Box::new(VirtIOBlockReq {
            type_:    VirtIOBlockReqType::Read as u32,
            reserved: 0,
            sector:   sector as u64,
        });

        let status: Box<u8> = Box::new(0xff); // device writes 0 on success
        let status_ptr = &*status as *const u8;

        let desc = unsafe { self.queue.desc.as_mut() };
        desc[0] = VirtqDesc {
            addr:  &*header as *const _ as u64,
            len:   core::mem::size_of::<VirtIOBlockReq>() as u32,
            flags: VirtqDescFlags::NEXT.bits(),
            next:  1,
        };

        desc[1] = VirtqDesc {
            addr:  buf.as_ptr() as u64,
            len:   BLOCK_SIZE as u32,
            flags: (VirtqDescFlags::NEXT | VirtqDescFlags::WRITE).bits(),
            next:  2,
        };

        desc[2] = VirtqDesc {
            addr:  status_ptr as u64,
            len:   1,
            flags: VirtqDescFlags::WRITE.bits(),
            next:  0,
        };

        // notify device
        let avail = unsafe { self.queue.avail.as_mut() };

        let avail_idx = avail.idx.read();
        avail.ring[avail_idx as usize % QUEUE_SIZE] = Volatile::from(0);

        core::sync::atomic::fence(Ordering::SeqCst);
        avail.idx.write(avail_idx + 1);

        unsafe {
            (*self.base).queue_notify.write(0);
        }

        // wait device
        loop {
            let used = unsafe { self.queue.used.read_volatile() };
            if used.idx.read() != self.used_idx {
                let id = used.ring[self.used_idx as usize % QUEUE_SIZE].id.read();
                debug!("[virtio] finished operation id: {}", id);
                break;
            }
        }
        self.used_idx = self.used_idx.wrapping_add(1);

        if unsafe { status_ptr.read_volatile() } == 0 {
            Ok(())
        } else {
            Err("IO error")
        }
    }

    // pub fn handle_interrupt(&mut self) {
    //     while self.used_idx != unsafe { self.queue.used.read().idx.read() } {
    //         let queue_used = unsafe { self.queue.used.read() };
    //         let _id = queue_used.ring[self.used_idx as usize % QUEUE_SIZE]
    //             .id
    //             .read();

    //         self.used_idx = self.used_idx.wrapping_add(1);
    //     }
    // }
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
