#![no_std]
#![no_main]
#![feature(str_from_raw_parts)]


use core::ffi::CStr;

use user::get_task_info;

#[macro_use]
extern crate user;

#[no_mangle]
fn main() -> i32 {
    let name:[u8; 20] = [0; 20];
    let id: usize = 0;
    let len = get_task_info(&id, &name, name.len());
    let name = CStr::from_bytes_until_nul(&name).unwrap();
    println!("get_task_info: id: {}, name: {}, len_of_name: {}", id, name.to_str().unwrap(), len);
    0
}
