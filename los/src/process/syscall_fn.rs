#![allow(unused)]

use alloc::vec::Vec;

use crate::{
    arch_relate::disable_kernel_interrupt,
    io::get_user_slice,
    process::{reschedule, restore_to_kernel, PROGRAM_MANAGER},
};

use super::{Process, RestoreBehavior};

#[inline]
pub(crate) fn sys_yield() -> RestoreBehavior {
    RestoreBehavior::Reschedule
}

#[inline]
pub(crate) fn exit(code: usize) -> ! {
    log!("Program exit with {}", code);
    PROGRAM_MANAGER.get().exit_current();
    reschedule();
    unreachable!()
}

pub(crate) fn sys_fork() -> RestoreBehavior {
    let mut mgr = PROGRAM_MANAGER.get();
    let pid;
    match mgr.current_program.as_mut() {
        Some(process) => {
            let forked = process.lock().fork();
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
    let token = PROGRAM_MANAGER.get().get_current_token();
    let slice = *get_user_slice(token, buf, len).get(0).unwrap();
    let mut mgr = PROGRAM_MANAGER.get();
    let str = core::str::from_utf8(slice).unwrap();
    match mgr.get_elf_by_name(str) {
        Some(data) => {
            match mgr.current_program.as_mut() {
                Some(process) => {
                    process.lock().exec(data);
                }
                None => unreachable!(),
            }
            RestoreBehavior::DirectReturn(0)
        }
        None => RestoreBehavior::DirectReturn(-1isize as usize),
    }
}
