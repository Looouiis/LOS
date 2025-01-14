#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(alloc_error_handler)]

extern crate alloc;
#[macro_use]
extern crate bitflags;

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
mod mem;

mod temp_test;

use batch::{run_program, PROGRAM_MANAGER};
use stack::{KERNAL_STACK_SIZE, TRAP_STACK};
use temp_test::{frame_allocator_test, remap_test};
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

#[no_mangle]
fn rust_main() {
    arch_relate::prepare_registers();
    clear_bss();
    println!("{BANNER}");
    timer::init();
    mem::init();
    println!();
    frame_allocator_test();
    remap_test();
    println!("trap_top: {:#x}", core::ptr::addr_of!(TRAP_STACK) as usize + KERNAL_STACK_SIZE);

    // temp_test::test_kernel_interrupt();

    PROGRAM_MANAGER.get().print_info();
    PROGRAM_MANAGER.get().init();
    let num = run_program();
    log!("arch_relate::run_program entered {} times", num);
    trace!("main trace");
    // HEAP_ALLOCATOR.check_leak();
    shutdown();
    // loop {}
}
