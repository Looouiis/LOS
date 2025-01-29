#![no_std]
#![no_main]

use alloc::string::String;
use user::{exec, fork, io::get_char, waitpid};

#[macro_use]
extern crate user;
extern crate alloc;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;

#[no_mangle]
fn main() -> i32 {
    let mut line: String = String::new();
    print!("> ");
    loop {
        let c = get_char();
        match c {
            LF | CR => {
                println!();
                if line == "exit" {
                    break;
                }
                let pid = fork();
                if pid == 0 {
                    if exec(line.as_str()) == -1 {
                        println!("Cann't find process");
                        return 0;
                    }
                } else {
                    let mut exit_code = 0;
                    waitpid(pid, &mut exit_code);
                    println!("usershell: process exit with {exit_code}");
                    println!();
                    print!("> ");
                }
                line.clear();
            }
            BS | DL => {
                print!("{} {}", BS as char, BS as char);
            }
            ch => {
                let char = ch as char;
                print!("{}", char);
                line.push(char);
            }
        }
    }
    0
}
