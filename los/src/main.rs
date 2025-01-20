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
mod config;
mod mem;
mod panic;
mod power;
mod process;
mod stack;
mod syscall;
mod timer;

mod temp_test;

use core::arch::global_asm;
use arch_relate::{disable_kernel_interrupt, timer::set_nxt_trigger};
use power::shutdown;
use process::{run_program, PROGRAM_MANAGER};
use temp_test::{frame_allocator_test, heap_test, remap_test};

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

    heap_test();
    frame_allocator_test();
    remap_test();

    PROGRAM_MANAGER.get().print_info();
    PROGRAM_MANAGER.get().init();
    PROGRAM_MANAGER.get().add_task("initproc");
    let num = run_program();
    log!("arch_relate::run_program entered {} times", num);
    trace!("main trace");
    shutdown();
    // loop {}
}
