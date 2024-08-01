#![no_std]
#![no_main]

use core::arch::asm;

mod panic;

#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    asm!(
        "   li x1, 100
        ",
        // options(noreturn)
    );
    loop{}
}
