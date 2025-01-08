#![no_std]
#![no_main]
#![feature(naked_functions)]

#[macro_use]
mod io;
#[macro_use]
mod arch_relate;
mod batch;
mod config;
mod panic;
mod power;
mod stack;
mod syscall;
mod timer;

use batch::{run_program, PROGRAM_MANAGER};
use core::arch::global_asm;
use power::shutdown;

// 由于_start与架构相关，所以具体请移步arch_relate模块

global_asm!(include_str!("link_program.S"));

const BANNER: &str = r#"
______                          _______________        
___  / ______________________  ____(_)__(_)__(_)_______
__  /  _  __ \  __ \  __ \  / / /_  /__  /__  /__  ___/
_  /___/ /_/ / /_/ / /_/ / /_/ /_  / _  / _  / _(__  ) 
/_____/\____/\____/\____/\__,_/ /_/  /_/  /_/  /____/  "#;

fn clear_bss() {
    extern "C" {
        fn __bss_start();
        fn __bss_end();
    }
    unsafe {
        for byte in __bss_start as usize..__bss_end as usize {
            (byte as *mut u8).write_volatile(0);
        }
    }
}

pub mod temp_test {
    use crate::arch_relate;

    static mut KERNEL_INTERRUPT_TRIGGERED: bool = false;

    /// 检查内核中断是否触发
    pub fn check_kernel_interrupt() -> bool {
        unsafe { (&raw mut KERNEL_INTERRUPT_TRIGGERED as *mut bool).read_volatile() }
    }

    /// 标记内核中断已触发
    pub fn trigger_kernel_interrupt() {
        unsafe {
            (&raw mut KERNEL_INTERRUPT_TRIGGERED as *mut bool).write_volatile(true);
        }
    }

    pub fn test_kernel_interrupt() {
        arch_relate::enable_kernel_interrupt();
        loop {
            if check_kernel_interrupt() {
                println!("kernel interrupt returned.");
                break;
            }
        }
        arch_relate::disable_kernel_interrupt();
    }
}

#[no_mangle]
fn rust_main() {
    arch_relate::prepare_registers();
    clear_bss();
    println!("{BANNER}");
    println!("Time Sharing Multitasking\n");
    timer::init();

    // temp_test::test_kernel_interrupt();

    PROGRAM_MANAGER.get().print_info();
    let num = run_program();
    log!("arch_relate::run_program entered {} times", num);
    trace!("main trace");
    shutdown();
    // loop {}
}
