use core::fmt::{self, Write};

use syscall::{read, write};

const STDIN: usize = 0;
const STDOUT: usize = 1;

pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write(STDOUT, s.as_bytes());
        Ok(())
    }
}

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

pub struct Stdin;

impl Stdin {
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, ()> {
        let res = read(STDIN, buf);
        if res < 0 {
            Err(())
        } else {
            Ok(res as usize)
        }
    }
}