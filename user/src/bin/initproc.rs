#![no_std]
#![no_main]

use user::{exec, fork, sys_yield, wait};

#[macro_use]
extern crate user;

#[no_mangle]
fn main() -> i32 {
    if fork() == 0 {
        exec("shell");
    } else {
        loop {
            let mut exit_code = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                sys_yield();
                continue;
            }
            println!("process {pid} exit with {exit_code}");
        }
    }
    0
}
