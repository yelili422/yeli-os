/// UART registers: http://byterunner.com/16550.html
use spin::Mutex;

use crate::{mem::UART0, sync::once_cell::OnceCell};

// Register offsets
const RHR: usize = 0; // receive holding register
const THR: usize = 0; // transmit holding register
const IER: usize = 1; // interrupt enable register
const FCR: usize = 2; // FIFO control register
const ISR: usize = 2; // interrupt status register
const LCR: usize = 3; // line control register
const LSR: usize = 5; // line status register

// Bit patterns
const IER_RX_ENABLE: u8 = 1 << 0;
const IER_TX_ENABLE: u8 = 1 << 1;
const FCR_FIFO_ENABLE: u8 = 1 << 0;
const FCR_FIFO_CLEAR: u8 = 3 << 1;
const LCR_EIGHT_BITS: u8 = 3 << 0;
const LCR_BAUD_LATCH: u8 = 1 << 7;
const LSR_RX_READY: u8 = 1 << 0;
const LSR_TX_IDLE: u8 = 1 << 5;

macro_rules! uart_reg {
    ($reg:expr) => {
        #[allow(unused_unsafe)]
        unsafe {
            ((UART0 + $reg) as *mut u8)
        }
    };
}

pub fn uart_init() {
    unsafe {
        // Disable interrupts
        uart_reg!(IER).write_volatile(0);

        // Special mode to set baud rate
        uart_reg!(LCR).write_volatile(LCR_BAUD_LATCH);

        // Set baud rate
        uart_reg!(0).write_volatile(0x03); // LSB for 38.4K
        uart_reg!(1).write_volatile(0x00); // MSB for 38.4K

        // Leave set-baud mode and set word length to 8 bits, no parity
        uart_reg!(LCR).write_volatile(LCR_EIGHT_BITS);

        // Reset and enable FIFOs
        uart_reg!(FCR).write_volatile(FCR_FIFO_ENABLE | FCR_FIFO_CLEAR);

        // Enable transmit and receive interrupts
        uart_reg!(IER).write_volatile(IER_TX_ENABLE | IER_RX_ENABLE);
    }

    UART.init(|| Mutex::new(Uart::new())).unwrap();
}

struct Uart {}

impl Uart {
    const fn new() -> Self {
        Uart {}
    }

    pub fn put_ch(&mut self, c: u8) {
        unsafe {
            while uart_reg!(LSR).read_volatile() & LSR_TX_IDLE == 0 {}
            uart_reg!(THR).write_volatile(c);
        }
    }

    fn get_ch(&mut self) -> Option<u8> {
        unsafe {
            if uart_reg!(LSR).read_volatile() & 0x01 == 0x01 {
                Some(uart_reg!(RHR).read_volatile())
            } else {
                None
            }
        }
    }
}

static UART: OnceCell<Mutex<Uart>> = OnceCell::new();

pub fn uart_put_ch(c: u8) {
    UART.get().expect("UART not initialized.").lock().put_ch(c);
}

pub fn uart_get_ch() -> Option<u8> {
    UART.get().expect("UART not initialized.").lock().get_ch()
}
