#![no_std]
#![feature(linkage)]

use core::panic::PanicInfo;

use syscall::sys_exit;

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
fn panic(_info: &PanicInfo) -> ! {
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

fn exit(exit_code: usize) -> ! {
    sys_exit(exit_code)
}
