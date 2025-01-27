use syscall_spec::*;

use crate::{
    io::{linux_write, sys_read},
    process::{
        syscall_fn::{exit, sys_exec, sys_fork, sys_yield},
        write_task, RestoreBehavior,
    },
};

pub(crate) fn syscall(id: usize, args: [usize; 3]) -> RestoreBehavior {
    match id {
        SYSCALL_WIRTE => linux_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => exit(args[0]),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8, args[1]),
        SYSCALL_READ => sys_read(args[0], args[1] as *mut u8, args[2]),
        GET_TASK_INFO => write_task(args[0] as *mut usize, args[1] as *mut u8, args[2]),
        _ => {
            log!("unsupported syscall: {}", id);
            exit(1);
        }
    }
}
