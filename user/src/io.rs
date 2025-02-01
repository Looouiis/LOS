use core::fmt::Write;

use crate::syscall::{sys_read, write};

struct Stdout;

const STDIN: usize = 0;
const STDOUT: usize = 1;

pub fn get_char() -> u8 {
    let mut c = [0u8; 1];
    sys_read(STDIN, &mut c);
    c[0]
}

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
        print!("\n");
    };
    ($($arg:tt)*) => {{
        $crate::io::print_fmt(core::format_args!($($arg)*));
        print!("\n");
    }};
}
