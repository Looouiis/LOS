#![allow(unused)]

use alloc::{slice, vec::Vec};
use syscall_spec::fs::FileFlags;

use crate::{
    arch_relate::disable_kernel_interrupt,
    fs::open_file,
    io::get_user_buf,
    mem::page_table::ROTable,
    process::{reschedule, switch_task, PROCESS_MANAGER},
};

use super::{Process, RestoreBehavior};

#[inline]
pub(crate) fn sys_yield() -> RestoreBehavior {
    RestoreBehavior::Reschedule
}

#[inline]
pub(crate) fn exit(code: usize) -> ! {
    log!("process exit with {}", code);
    PROCESS_MANAGER.get().exit_current(code as i32);
    reschedule();
    unreachable!()
}

pub(crate) fn sys_fork() -> RestoreBehavior {
    let mut mgr = PROCESS_MANAGER.get();
    let pid;
    match mgr.current_program.as_mut() {
        Some(process) => {
            let forked = Process::fork(&process);
            pid = *(forked.lock().pid);
            mgr.process.push_front(forked);
        }
        None => unreachable!(),
    }
    RestoreBehavior::DirectReturn(pid)
}

pub(crate) fn sys_waitpid(pid: usize, exit_code: *mut i32) -> RestoreBehavior {
    todo!()
}

pub(crate) fn sys_exec(buf: *const u8, len: usize) -> RestoreBehavior {
    let mut mgr = PROCESS_MANAGER.get();
    let table = ROTable::from_token(mgr.get_current_token());
    let slice_vec = get_user_buf(&table, buf, len);
    let slice = match slice_vec.get(0) {
        Some(slice) => slice,
        None => return RestoreBehavior::DirectReturn(-1isize as usize),
    };
    let str = core::str::from_utf8(slice).unwrap();
    match /* mgr.get_elf_by_name(str) */ open_file(str, FileFlags::RDONLY).map(|file| file.read_all()) {
        Some(data) => {
            match mgr.current_program.as_mut() {
                Some(process) => {
                    process.lock().exec(data.as_slice());
                }
                None => unreachable!(),
            }
            RestoreBehavior::DirectReturn(0)
        }
        None => {
            log!("can't find name {str}");
            RestoreBehavior::DirectReturn(-1isize as usize)
        }
    }
}
