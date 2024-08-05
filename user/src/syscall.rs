use core::arch::asm;

pub fn write(fd: usize, buf: &[u8]) {
    sys_write(fd, buf.as_ptr(), buf.len());
}

const SYSCALL_WIRTE: usize = 64;
const SYSCALL_EXIT: usize = 93;

fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") args[0] => ret,
            in("a1") args[1],
            in("a2") args[2],
            in("a7") id,
        )
    }
    ret
}

/// 功能：将内存中缓冲区中的数据写入文件。
/// 
/// 参数：`fd` 表示待写入文件的文件描述符；
///      `buf` 表示内存中缓冲区的起始地址；
///      `len` 表示内存中缓冲区的长度。
/// 
/// 返回值：返回成功写入的长度。
/// 
/// syscall ID：64
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    syscall(SYSCALL_WIRTE, [fd, buf as usize, len])
}

/// 功能：退出应用程序并将返回值告知批处理系统。
/// 
/// 参数：`exit_code` 表示应用程序的返回值。
/// 
/// 返回值：该系统调用不应该返回。
/// 
/// syscall ID：93
pub fn sys_exit(exit_code: usize) -> ! {
    syscall(SYSCALL_EXIT, [exit_code, 0, 0]);
    unreachable!()
}