use alloc::vec::Vec;
use core::fmt::Write;

use crate::{
    arch_relate::ecall::{getch, putch},
    fs::{self, File},
    mem::{
        address::{StepByOne, VirAddr},
        page_table::ROTable,
    },
    process::switch_task,
};

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
        $crate::arch_relate::ecall::putch('\n' as usize);
    };
    ($($arg:tt)*) => {{
        $crate::io::print_fmt(core::format_args!($($arg)*));
        $crate::arch_relate::ecall::putch('\n' as usize);
    }};
}

// const STDOUT: usize = 1;
// const STDIN: usize = 0;

pub(crate) fn get_user_buf(
    page_table: &ROTable,
    ptr: *const u8,
    len: usize,
) -> Vec<&'static mut [u8]> {
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirAddr::from(start);
        let mut vpn = start_va.floor_to_vpn();
        let ppn = page_table.vpn_to_pte(vpn).unwrap().ppn();
        vpn.step();
        let end_va = VirAddr::from(vpn).min(VirAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

pub(crate) struct Stdio;

impl File for Stdio {
    fn readable(&self) -> bool {
        true
    }

    fn writeable(&self) -> bool {
        true
    }

    fn read(&self, buf: fs::UserBuffer) -> usize {
        let mut len = 0;
        for slice in buf {
            for pa in slice {
                let mut ch;
                loop {
                    ch = getch();
                    if ch == 0 {
                        switch_task();
                    } else {
                        break;
                    }
                }
                *pa = ch;
                len += 1;
            }
        }
        len
    }

    fn write(&self, buf: fs::UserBuffer) -> usize {
        let mut len = 0;
        for slice in buf {
            let str = core::str::from_utf8(&slice).unwrap();
            print!("{}", str);
            len += str.len();
        }
        len
    }
}
