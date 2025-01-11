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
use temp_test::frame_allocator_test;
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

fn print_kernel_info() {
    unsafe extern "C" {
        fn __text_start();
        fn __text_end();
        fn __rodata_start();
        fn __rodata_end();
        fn __data_start();
        fn __data_end();
        fn __bss_end();
    }
    println!(".text [{:#x}, {:#x})", __text_start as usize, __text_end as usize);
    println!(".rodata [{:#x}, {:#x})", __rodata_start as usize, __rodata_end as usize);
    println!(".data [{:#x}, {:#x})", __data_start as usize, __data_end as usize);
    println!(
        ".bss [{:#x}, {:#x})",
        __data_end as usize, __bss_end as usize
    );
}

#[no_mangle]
fn rust_main() {
    arch_relate::prepare_registers();
    clear_bss();
    println!("{BANNER}");
    timer::init();
    mem::init();
    println!();
    print_kernel_info();
    frame_allocator_test();

    // temp_test::test_kernel_interrupt();

    PROGRAM_MANAGER.get().print_info();
    let num = run_program();
    log!("arch_relate::run_program entered {} times", num);
    trace!("main trace");
    shutdown();
    // loop {}
}
