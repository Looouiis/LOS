use core::fmt::Write;

use crate::syscall::write;

struct Stdout;

const STDOUT: usize = 1;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write(STDOUT, s.as_bytes());
        Ok(())
    }
}

pub fn print_fmt(args: core::fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::io::print_fmt(core::format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::syscall::putch('\n' as usize);
    };
    ($($arg:tt)*) => {{
        $crate::io::print_fmt(core::format_args!($($arg)*));
        print!("\n");
    }};
}