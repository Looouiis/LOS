use core::arch::asm;

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

const SYSCALL_WIRTE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const GET_TASK_INFO: usize = 38;

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
