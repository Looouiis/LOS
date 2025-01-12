#![allow(unused)]

use alloc::vec::Vec;

use crate::{arch_relate, mem::{address::VirAddr, allocator::{frame::FrameTracker, FRAME_ALLOCATOR}, memory_set::KERNEL_SPACE}};

static mut KERNEL_INTERRUPT_TRIGGERED: bool = false;

/// 检查内核中断是否触发
pub fn check_kernel_interrupt() -> bool {
    unsafe { (&raw mut KERNEL_INTERRUPT_TRIGGERED as *mut bool).read_volatile() }
}

/// 标记内核中断已触发
pub fn trigger_kernel_interrupt() {
    unsafe {
        (&raw mut KERNEL_INTERRUPT_TRIGGERED as *mut bool).write_volatile(true);
    }
}

pub fn test_kernel_interrupt() {
    arch_relate::enable_kernel_interrupt();
    loop {
        if check_kernel_interrupt() {
            println!("kernel interrupt returned.");
            break;
        }
    }
    arch_relate::disable_kernel_interrupt();
}

pub fn heap_test() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    extern "C" {
        fn __rodata_start();
        fn __rodata_end();
    }
    let bss_range = __rodata_start as usize..__rodata_end as usize;
    let a = Box::new(5);
    assert_eq!(*a, 5);
    assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for (i, val) in v.iter().take(500).enumerate() {
        assert_eq!(*val, i);
    }
    assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    println!("heap_test passed!");
}

pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();
    for _i in 0..5 {
        let frame = FRAME_ALLOCATOR.alloc().unwrap();
        println!("{:?}", frame.ppn.0);
        v.push(frame);
    }
    v.clear();
    for _i in 0..5 {
        let frame = FRAME_ALLOCATOR.alloc().unwrap();
        println!("{:?}", frame.ppn.0);
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}

pub fn remap_test() {
    extern "C" {
        fn __text_start();
        fn __text_end();
        fn __rodata_start();
        fn __rodata_end();
        fn __data_start();
        fn __data_end();
        fn __bss_start_with_stack();
        fn __bss_end();
        fn __kernel_end();
    }
    let mut kernel_space = KERNEL_SPACE.get();
    let mid_text: VirAddr = ((__text_start as usize + __text_end as usize) / 2).into();
    let mid_rodata: VirAddr = ((__rodata_start as usize + __rodata_end as usize) / 2).into();
    let mid_data: VirAddr = ((__data_start as usize + __data_end as usize) / 2).into();
    assert_eq!(
        kernel_space.page_table.vpn_to_pte(mid_text.floor_to_vpn()).unwrap().writable(),
        false
    );
    assert_eq!(
        kernel_space.page_table.vpn_to_pte(mid_rodata.floor_to_vpn()).unwrap().writable(),
        false,
    );
    assert_eq!(
        kernel_space.page_table.vpn_to_pte(mid_data.floor_to_vpn()).unwrap().executable(),
        false,
    );
    println!("remap_test passed!");
}
