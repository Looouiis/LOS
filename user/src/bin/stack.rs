#![no_std]
#![no_main]

use core::{arch::asm, ptr};

#[macro_use]
extern crate user;


fn print_stack_trace() {
    let mut fp: *const usize;
    unsafe {
        asm!(
            "mv t0, fp",
            out("t0") fp
        );
        while fp != ptr::null() {
            println!("back to fp: 0x{:016x}", fp as usize);
            fp = fp.sub(2).read_volatile() as *const usize;
        }
    }
}

#[no_mangle]
fn a() {
    print_stack_trace();
}

#[no_mangle]
fn main() -> i32 {
    println!("Stack trace Program");
    // print_stack_trace();
    a();
    0
}