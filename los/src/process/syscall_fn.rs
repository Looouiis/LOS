#![allow(unused)]

use crate::{io::get_user_slice, process::{reschedule, restore_to_kernel, set_nxt_trigger, PROGRAM_MANAGER}};

use super::{Process, RestoreBehavior};

#[inline]
pub(crate) fn sys_yield() -> RestoreBehavior {
    RestoreBehavior::Reschedule
}

#[inline]
pub(crate) fn exit(code: usize) -> ! {
    log!("Program exit with {}", code);
    PROGRAM_MANAGER.get().exit_current();
    reschedule()
}

pub(crate) fn sys_fork() -> RestoreBehavior {
    let mut mgr = PROGRAM_MANAGER.get();
    let mut guard = mgr.current_program.lock();
    let pid;
    match guard.as_mut() {
        Some(process) => {
            let forked = process.fork();
            pid = *(forked.pid);
            drop(guard);
            mgr.process.push_front(forked);
        },
        None => unreachable!(),
    }
    RestoreBehavior::DirectReturn(pid)
}

pub(crate) fn sys_waitpid(pid: usize, exit_code: *mut i32) -> RestoreBehavior {
    todo!()
}

pub(crate) fn sys_exec(buf: *const u8, len: usize) -> RestoreBehavior {
    let token = PROGRAM_MANAGER.get().get_current_token();
    let slice = *get_user_slice(token, buf, len).get(0).unwrap();
    let mgr = PROGRAM_MANAGER.get();
    let str = core::str::from_utf8(slice).unwrap();
    match mgr.get_elf_by_name(str) {
        Some(data) => {
            let mut guard = mgr.get_process().lock();
            match guard.as_mut() {
                Some(process) => {
                    process.exec(data);
                },
                None => unreachable!(),
            }
        },
        None => {
            log!("can't find target elf by name {str}");
            drop(mgr);
            exit(1);
        },
    }
    RestoreBehavior::DirectReturn(0)
}
