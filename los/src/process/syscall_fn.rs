#![allow(unused)]

use crate::process::{reschedule, restore_to_kernel, set_nxt_trigger, PROGRAM_MANAGER};

use super::RestoreBehavior;

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
    todo!()
}

pub(crate) fn sys_waitpid(pid: usize, exit_code: *mut i32) -> RestoreBehavior {
    todo!()
}

pub(crate) fn sys_exec(path: &str) -> RestoreBehavior {
    todo!()
}
