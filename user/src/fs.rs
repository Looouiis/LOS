pub use syscall_spec::fs::FileFlags;

use crate::syscall::{sys_close, sys_open, sys_read};

pub fn open(path: &str, flags: FileFlags) -> isize {
    sys_open(path, flags.bits())
}

pub fn close(fd: usize) -> isize {
    sys_close(fd)
}

pub fn read(fd: usize, buffer: &mut [u8]) -> isize {
    sys_read(fd, buffer)
}
