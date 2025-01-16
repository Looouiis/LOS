#![no_std]

pub const SYSCALL_WIRTE: usize = 64;
pub const SYSCALL_EXIT: usize = 93;
pub const SYSCALL_YIELD: usize = 124;
pub const SYSCALL_FORK: usize = 220;
pub const SYSCALL_WAIT_PID: usize = 260;
pub const SYSCALL_EXEC: usize = 221;
pub const GET_TASK_INFO: usize = 38;
