use core::arch::asm;
use syscall_spec::*;

/// 功能：将内存中缓冲区中的数据写入文件。
///
/// 参数：
///
/// `fd` 表示待写入文件的文件描述符；
///
/// `buf` 表示内存中缓冲区的起始地址；
///
/// `len` 表示内存中缓冲区的长度。
///
/// 返回值：返回成功写入的长度。
///
/// syscall ID：64
pub fn write(fd: usize, buf: &[u8]) {
    sys_write(fd, buf.as_ptr(), buf.len());
}

fn syscall(id: usize, args: [usize; 3]) -> usize {
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
/// 参数：
///
/// `fd` 表示待写入文件的文件描述符；
///
/// `buf` 表示内存中缓冲区的起始地址；
///
/// `len` 表示内存中缓冲区的长度。
///
/// 返回值：返回成功写入的长度。
///
/// syscall ID：64
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> usize {
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

/// 功能：主动出让cpu
///
/// 参数：无
///
/// 返回值：0
///
/// syscall ID：124
pub fn sys_yield() -> usize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

/// 功能：获取应用程序在LOS中的task_id与name（长度不超过20）
///
/// 参数：
///
/// `id` 接收task_id，被LOS修改。
///
/// `name` 接收name，被LOS修改，放入的值遵循C字符串的风格。
///
/// `len` 指定接受的字符串长度（长度不包含`\0`）
///
/// 返回值：实际拷贝的字节数
///
/// # Safety
///
/// 请自行确保`\0`正确性，与C中的strcpy一样，此函数理应只负责复制字符串部分，但是对于实际name的长度小于len的情况，LOS会帮你加一个`\0`
///
/// syscall ID：38
pub fn sys_task_info(id: *const usize, name: *const u8, len: usize) -> usize {
    syscall(GET_TASK_INFO, [id as usize, name as usize, len]) as usize
    // unreachable!()
}

/// 功能：当前进程等待一个子进程变为僵尸进程，回收其全部资源并收集其返回值。
///
/// 参数：
///
/// `pid`` 表示要等待的子进程的进程 ID，如果为 -1 的话表示等待任意一个子进程；
///
/// `exit_code` 表示保存子进程返回值的地址，如果这个地址为 0 的话表示不必保存。
///
/// 返回值：如果要等待的子进程不存在则返回 -1；否则如果要等待的子进程均未结束则返回 -2；
///         否则返回结束的子进程的进程 ID。
///
/// syscall ID：260
pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall(SYSCALL_WAIT_PID, [pid as usize, exit_code as usize, 0]) as isize
}

/// 功能：当前进程 fork 出来一个子进程。
///
/// 返回值：对于子进程返回 0，对于当前进程则返回子进程的 PID 。
///
/// syscall ID：220
pub fn sys_fork() -> usize {
    syscall(SYSCALL_FORK, [0, 0, 0])
}

/// 功能：将当前进程的地址空间清空并加载一个特定的可执行文件，返回用户态后开始它的执行。
///
/// 参数：`path` 给出了要加载的可执行文件的名字；
///
/// 返回值：如果出错的话（如找不到名字相符的可执行文件）则返回 -1，否则不应该返回。
///
/// syscall ID：221
pub fn sys_exec(str: &str) -> isize {
    syscall(SYSCALL_EXEC, [str.as_ptr() as usize, 0, 0]) as isize
}

/// 功能：从文件中读取一段内容到缓冲区。
///
/// 参数：fd 是待读取文件的文件描述符，切片 buffer 则给出缓冲区。
///
/// 返回值：如果出现了错误则返回 -1，否则返回实际读到的字节数。
///
/// syscall ID：63
pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall(SYSCALL_READ, [fd, buffer.as_ptr() as usize, buffer.len()]) as isize
}
