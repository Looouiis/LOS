use alloc::vec::Vec;

use crate::{arch_relate::ecall::putch, batch::{RestoreBehavior, PROGRAM_MANAGER}, mem::{address::{StepByOne, VirAddr}, page_table::PageTable}};
use core::fmt::Write;

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

const STDOUT: usize = 1;

pub(crate) fn translate_to_kernel_ppn(satp: usize, ptr: *const u8, len: usize) -> Vec<&'static [u8]> {
    let page_table = PageTable::from_satp(satp);
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
            v.push(&ppn.get_bytes_array()[start_va.page_offset() ..]);
        }
        else {
            v.push(&ppn.get_bytes_array()[start_va.page_offset() .. end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

pub(crate) fn linux_write(fd: usize, buf: *const u8, len: usize) -> RestoreBehavior {
    match fd {
        STDOUT => {
            let buffer = translate_to_kernel_ppn(PROGRAM_MANAGER.get().get_current_satp(), buf, len);
            for slice in buffer {
                let str = core::str::from_utf8(slice).unwrap();
                print!("{}", str);
            }
            RestoreBehavior::DirectReturn(len)
        }
        _ => panic!("unsupported fd type: {}", fd),
    }
}
