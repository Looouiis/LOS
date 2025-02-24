#![no_std]

pub mod fs;

pub const SYSCALL_WIRTE: usize = 64;
pub const SYSCALL_EXIT: usize = 93;
pub const SYSCALL_YIELD: usize = 124;
pub const SYSCALL_FORK: usize = 220;
pub const SYSCALL_WAIT_PID: usize = 260;
pub const SYSCALL_EXEC: usize = 221;
pub const SYSCALL_READ: usize = 63;
pub const SYSCALL_OPEN: usize = 56;
pub const SYSCALL_CLOSE: usize = 57;
pub const GET_TASK_INFO: usize = 38;
