use crate::{batch::{exit, write_task}, io::linux_write};

pub(crate) const SYSCALL_WIRTE: usize = 64;
pub(crate) const SYSCALL_EXIT: usize = 93;
pub(crate) const GET_TASK_INFO: usize = 38;

pub(crate) fn syscall(id: usize, args: [usize; 3]) -> usize {
    match id {
        SYSCALL_WIRTE => linux_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => exit(args[0]),
        GET_TASK_INFO => write_task(args[0] as *mut usize, args[1] as *mut u8, args[2]),
        _ => panic!("unsupported syscall: {}", id)
    }
}
