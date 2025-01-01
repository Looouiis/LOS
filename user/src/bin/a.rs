#![no_std]
#![no_main]

#[macro_use]
extern crate user;

use user::sys_yield;

#[no_mangle]
fn main() -> i32 {
    for i in 1..=3 {
        println!("program a: {}/3 times", i);
        sys_yield();
    }
    0
}
