#![no_std]
#![no_main]

use user::{exec, fork, wait};

#[macro_use]
extern crate user;

#[no_mangle]
fn main() -> i32 {
    if fork() == 0 {
        exec("usershell");
    } else {
        loop {
            let mut exit_code = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                println!("initproc: all process finish");
                break;
            }
            println!("initproc: process {pid} exit with {exit_code}");
        }
    }
    0
}
