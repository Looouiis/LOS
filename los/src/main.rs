#![no_std]
#![no_main]
#![feature(naked_functions, asm_const)]

#[macro_use]
mod io;
#[macro_use]
mod arch_relate;
mod panic;
mod stack;
mod power;
mod syscall;
mod batch;

use core::arch::global_asm;
use batch::{run_app, APP_MANAGER};
use power::shutdown;

// 由于_start与架构相关，所以具体请移步arch_relate模块

global_asm!(include_str!("link_app.S"));

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

#[no_mangle]
fn rust_main() {
    clear_bss();
    APP_MANAGER.get().print_info();
    let num = run_app();
    log!("run {} app", num);
    trace!("main trace");
    shutdown();
    // loop {}
}
