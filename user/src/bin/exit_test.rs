#![no_std]
#![no_main]

use user::{fork, sys_yield};

#[macro_use]
extern crate user;

#[no_mangle]
fn main() -> i32 {
    if fork() == 0 {
        println!("exit_test: initproc should manage this child process as its child");
        for _i in 0..90000 {
            sys_yield();
        }
        println!("exit_test: child exit");
    } else {
        println!("exit_test: father process will exit immediately");
    }
    0
}
