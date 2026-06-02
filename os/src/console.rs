use crate::drivers::chardev::{CharDevice, UART};
use core::fmt::{self, Write};

struct Stdout;

impl Write for Stdout {
    ///
    /// 输出字符串
    ///
    /// @author: tryte
    ///
    /// @date: 2025/11/19
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            UART.write(c as u8);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(,$($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(,$($arg)+)?));
    };
}

#[macro_export]
macro_rules! println {
     ($fmt: literal $(,$($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(,$($arg)+)?));
    };
}
