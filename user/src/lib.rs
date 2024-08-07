#![no_std]
#![feature(linkage)]

use core::{panic::PanicInfo, ptr};

use syscall::{sys_exit, sys_task_info};

pub mod io;
pub mod syscall;

#[no_mangle]
#[link_section = ".text.entry"]
fn _start() {
    clear_bss();
    exit(main() as usize);
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> isize {
    panic!("Cannot find main!");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(loc) = info.location() {
        println!("[user]: Panicked at {} {}: {}",
                loc.file(),
                loc.line(),
                info.message());
    }
    exit(1)
}

fn clear_bss() {
    extern "C" {
        fn __bss_start();
        fn __bss_end();
    }
    unsafe {
        for byte in __bss_start as usize .. __bss_end as usize {
            (byte as *mut u8).write_volatile(0);
        }
    }
}

/// 功能：退出应用程序并将返回值告知批处理系统。
/// 
/// 参数：`exit_code` 表示应用程序的返回值。
/// 
/// 返回值：该系统调用不应该返回。
/// 
/// syscall ID：93
fn exit(exit_code: usize) -> ! {
    sys_exit(exit_code)
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
pub fn get_task_info(id: &usize, name: &[u8], len: usize) -> usize {
    sys_task_info(ptr::addr_of!(*id), name.as_ptr(), len)
}
