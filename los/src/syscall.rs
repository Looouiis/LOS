use crate::{batch::exit, io::linux_write};

pub(crate) const SYSCALL_WIRTE: usize = 64;
pub(crate) const SYSCALL_EXIT: usize = 93;
// pub(crate) const GET_TASK_INFO: usize = 

pub(crate) fn syscall(id: usize, args: [usize; 3]) -> usize {
    match id {
        SYSCALL_WIRTE => linux_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => exit(args[0]),
        _ => panic!("unsupported syscall: {}", id)
    }
}
