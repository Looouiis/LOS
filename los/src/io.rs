use alloc::vec::Vec;
use core::fmt::Write;

use crate::{
    arch_relate::ecall::{getch, putch},
    mem::{
        address::{StepByOne, VirAddr},
        page_table::ROTable,
    },
    process::{switch_task, RestoreBehavior},
    PROGRAM_MANAGER,
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

const STDOUT: usize = 1;
const STDIN: usize = 0;

pub(crate) fn get_user_slice(token: usize, ptr: *const u8, len: usize) -> Vec<&'static [u8]> {
    let page_table = ROTable::from_token(token);
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
            v.push(&ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

pub(crate) fn linux_write(fd: usize, buf: *const u8, len: usize) -> RestoreBehavior {
    match fd {
        STDOUT => {
            let slice = get_user_slice(PROGRAM_MANAGER.get().get_current_token(), buf, len);
            for s in slice {
                let str = core::str::from_utf8(s).unwrap();
                print!("{}", str);
            }
            RestoreBehavior::DirectReturn(len)
        }
        _ => panic!("unsupported fd type: {}", fd),
    }
}

pub(crate) fn sys_read(fd: usize, buf: *mut u8, len: usize) -> RestoreBehavior {
    let mut cnt = 0;
    match fd {
        STDIN => {
            let token = PROGRAM_MANAGER.get().get_current_token();
            let page_table = ROTable::from_token(token);
            for offset in 0..len {
                let mut ch;
                loop {
                    ch = getch();
                    if ch == 0 {
                        switch_task();
                    } else {
                        break;
                    }
                }
                unsafe {
                    // buf.add(offset).write_volatile(ch);
                    let va = VirAddr::from(buf.add(offset) as usize);
                    let offset = va.page_offset();
                    let vpn = va.floor_to_vpn();
                    let slice = &mut page_table.vpn_to_pte(vpn).unwrap().ppn().get_bytes_array()
                        [offset..offset + 1];
                    slice[0] = ch;
                    cnt += 1;
                }
            }
        }
        _ => panic!("unsupported fd type: {}", fd),
    }
    RestoreBehavior::DirectReturn(cnt)
}
