//! The text buffer starts at physical address `0xb8000` and contains the 
//! characters displayed on screen. It has 25 rows and 80 columns.
//! Each screen character has the following format:
//!
//! Bit(s) | Value
//! ------ | ----------------
//! 0-7    | ASCII code point
//! 8-11   | Foreground color
//! 12-14  | Background color
//! 15     | Blink
//!
//! The following colors are available:
//!
//! Number | Color      | Number + Bright Bit | Bright Color
//! ------ | ---------- | ------------------- | -------------
//! 0x0    | Black      | 0x8                 | Dark Gray
//! 0x1    | Blue       | 0x9                 | Light Blue
//! 0x2    | Green      | 0xa                 | Light Green
//! 0x3    | Cyan       | 0xb                 | Light Cyan
//! 0x4    | Red        | 0xc                 | Light Red
//! 0x5    | Magenta    | 0xd                 | Pink
//! 0x6    | Brown      | 0xe                 | Yellow
//! 0x7    | Light Gray | 0xf                 | White
//!
//! Bit 4 is the _bright bit_, which turns for example blue into light blue. 
//! It is unavailable in background color as the bit is used to control 
//! if the text should blink. If you want to use a light background color 
//! (e.g. white) you have to disable blinking through a 
//! [BIOS function][disable blinking].

use core::fmt;
use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    // TODO:
    // To get synchronized interior mutability, users of the standard
    // library can use Mutex. It provides mutual exclusion by blocking
    // threads when the resource is already locked. 
    // But our basic kernel does not have any blocking support
    // or even a concept of threads, so we can't use it either.
    // However there is a really basic kind of mutex in computer science
    // that requires no operating system features: the spinlock.
    // Instead of blocking, the threads simply try to lock it again and again
    // in a tight loop and thus burn CPU time until the mutex is free again.
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}


#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Because of the repr(u8) attribute each enum variant is stored as an u8.
// Actually 4 bits would be sufficient, but Rust doesn't have an u4 type.
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// It guarantees that the struct's fields are laid out exactly like 
// in a C struct and thus guarantees the correct field ordering. 
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}


const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;


#[repr(transparent)]
struct Buffer {
    // The problem is that we only write to the Buffer and never 
    // read from it again. The compiler doesn't know that we really
    // access VGA buffer memory (instead of normal RAM) and knows
    // nothing about the side effect that some characters appear
    // on the screen. So it might decide that these writes are unnecessary
    // and can be omitted. 
    // To avoid this erroneous optimization, we need to specify
    // these writes as volatile. `Volatile` tells the compiler that
    // the write has side effects and should not be optimized away.
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}


/// A writer type that allows writing ASCII bytes and strings to an underlying `Buffer`.
///
/// Wraps lines at `BUFFER_WIDTH`. Supports newline characters and implements the
/// `core::fmt::Write` trait.
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

impl Writer {
    /// Writes an ASCII byte to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    /// Shifts all lines one line up and clears the last row.
    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    /// Clears a row by overwriting it with blank characters.
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    /// Writes the given ASCII string to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character. Does **not**
    /// support strings with non-ASCII characters, since they can't be printed in the VGA text
    /// mode.
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }

        }
    }
}


/// Like the `print!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Prints the given formatted string to the VGA text buffer
/// through the global `WRITER` instance.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
