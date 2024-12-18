use core::fmt::{self, Write};

use crate::{drivers::uart::uart_put_ch, fs::File};

struct Stdout;

impl File for Stdout {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, ()> {
        panic!("Cannot read from stdout");
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, ()> {
        unimplemented!()
    }
}

impl fmt::Write for Stdout {
    /// Prints a string, which can contain non-ASCII characters.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut buffer = [0u8; 4];

        for c in s.chars() {
            for code_point in c.encode_utf8(&mut buffer).as_bytes().iter() {
                // // The `console_putchar` sbi call accepts one 'u8` to print
                // // the characters actually. Therefore, if there are non-ASCII
                // // characters in the string, we need to be in utf-8 encoding
                // // call `console_putchar` once for each `u8`.
                // crate::intr::sbi::console_putchar(*code_point);

                uart_put_ch(*code_point);
            }
        }
        Ok(())
    }
}

/// Prints formatted string by [`core::format_args!`].
pub fn _print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::_print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::_print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! dbg {
    () => {
        $crate::console::_print(format_args!("[DEBUG] File: {}, Line: {}\n", file!(), line!()));
    };
}

pub struct HexDump<'a>(pub &'a [u8]);

impl<'a> fmt::Display for HexDump<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.0.chunks(16) {
            for byte in chunk {
                write!(f, "{:02X} ", byte)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
