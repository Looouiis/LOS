use core::fmt::Write;
use crate::arch_relate::ecall::putch;

struct Stdout;

#[macro_use]
mod logging {
    macro_rules! log {
        () => {
            $crate::syscall::putch('\n' as usize);
        };
        ($($arg:tt)*) => {{
            print!("\x1b[32m[kernel]: ");
            $crate::io::print_fmt(core::format_args!($($arg)*));
            print!("\x1b[0m\n");
        }};
    }

    macro_rules! trace {
        () => {
            $crate::syscall::putch('\n' as usize);
        };
        ($($arg:tt)*) => {{
            print!("\x1b[90m[kernel]: ");
            $crate::io::print_fmt(core::format_args!($($arg)*));
            print!("\x1b[0m\n");
        }};
    }
}

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        s.chars().for_each(|ch| {
            putch(ch as usize);
        });
        Ok(())
    }
}

pub(crate) fn print_fmt(args: core::fmt::Arguments) {
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
        $crate::arch_relate::ecall::putch('\n' as usize);
    }};
}

const STDOUT: usize = 1;

pub(crate) fn linux_write(fd: usize, buf: *const u8, len: usize) -> usize {
    match fd {
        STDOUT => {
            let slice = unsafe {
                core::slice::from_raw_parts(buf, len)
            };
            let str = core::str::from_utf8(slice).unwrap();
            print!("{}", str);
            len
        }
        _ => panic!("unsupported fd type: {}", fd)
    }
}

