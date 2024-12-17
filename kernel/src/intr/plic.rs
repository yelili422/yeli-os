use log::{debug, info};

use super::cpu_id;
use crate::{drivers::virtio::handle_virtio_interrupt, mem::PLIC_BASE};

#[repr(u32)]
#[derive(Debug)]
pub enum IRQ {
    UART0  = 10,
    VIRTIO = 1,
}

impl From<u32> for IRQ {
    fn from(value: u32) -> Self {
        unsafe { core::mem::transmute(value) }
    }
}

macro_rules! plic_irq_senable {
    ($hart_id:expr) => {
        *((crate::mem::PLIC_BASE + 0x2080 + ($hart_id * 0x100)) as *mut u32)
    };
}

macro_rules! plic_irq_spriority {
    ($hart_id:expr) => {
        *((crate::mem::PLIC_BASE + 0x201000 + ($hart_id * 0x2000)) as *mut u32)
    };
}

macro_rules! plic_sclaim {
    ($hart_id:expr) => {
        *((crate::mem::PLIC_BASE + 0x201004 + ($hart_id * 0x2000)) as *mut u32)
    };
}

pub unsafe fn plic_init() {
    // let hart = cpu_id();
    let hart = 0;

    debug!("init plic hart: {}", hart);

    // TODO: enable virtio interrupt
    // set_irq(IRQ::VIRTIO, 1);

    // enable irq for this hart in S-mode
    plic_irq_senable!(hart) |= 1 << IRQ::VIRTIO as u32;

    // set this hart's S-mode threshold to 0
    plic_irq_spriority!(hart) = 0;
}

unsafe fn set_irq(irq: IRQ, value: u32) {
    *((PLIC_BASE + (irq as usize * 4)) as *mut u32) = value;
}

pub fn handle_plic() {
    let hart_id = cpu_id();
    let irq = unsafe { plic_sclaim!(hart_id) };

    info!("Received PLIC interrupt: irq: {}, hart_id: {}", irq, hart_id);
    match IRQ::from(irq) {
        IRQ::VIRTIO => handle_virtio_interrupt(),
        _ => unimplemented!(),
    }
}
