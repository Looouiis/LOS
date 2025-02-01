use bitflags::bitflags;

use crate::syscall::{sys_close, sys_open, sys_read};

bitflags! {
    pub struct FileFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1;
        const RDWR = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}

pub fn open(path: &str, flags: FileFlags) -> isize {
    sys_open(path, flags.bits())
}

pub fn close(fd: usize) -> isize {
    sys_close(fd)
}

pub fn read(fd: usize, buffer: &mut [u8]) -> isize {
    sys_read(fd, buffer)
}
