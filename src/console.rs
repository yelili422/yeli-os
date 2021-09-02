
use core::fmt::{self, Write};

use crate::syscall::sbi::console_putchar;



struct Stdout;

impl Write for Stdout {
    /// 打印一个字符串
    ///
    /// [`console_putchar`] sbi 调用每次接受一个 `usize`，但实际上会把它作为 `u8` 来
    /// 打印字符。因此，如果字符串中存在非 ASCII 字符，需要在 utf-8 编码下，
    /// 对于每一个 `u8` 调用一次 [`console_putchar`]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut buffer = [0u8; 4];
        for c in s.chars() {
            for code_point in c.encode_utf8(&mut buffer).as_bytes().iter() {
                console_putchar(*code_point as usize);
            }
        }
        Ok(())
    }
}

/// 打印由 [`core::format_args!`] 格式化后的数据
/// 
/// [`print!`] 和 [`println!`] 宏都将展开成此函数
/// 
/// [`core::format_args!`]: https://doc.rust-lang.org/nightly/core/macro.format_args.html
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