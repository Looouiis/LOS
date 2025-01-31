use core::arch::{asm, naked_asm};
use riscv::register::sstatus;
use trap::ProcessContext;

use crate::{arch_relate, config::TRAMPOLINE, process::PROCESS_MANAGER, stack};

#[macro_use]
pub(crate) mod trap;
pub(crate) mod ecall;
pub(crate) mod timer;
#[macro_use]
pub(crate) mod syscall_handler;

#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    use stack::{KERNAL_STACK, KERNAL_STACK_SIZE};
    asm!(
        "   la      sp, {stack_btn}
            li      t0, {stack_size}
            add     sp, sp, t0
            la      t0, {trap_handler}
            csrw    stvec, t0
            j       rust_main
        ",
        stack_btn = sym KERNAL_STACK,
        stack_size = const KERNAL_STACK_SIZE,
        trap_handler = const TRAMPOLINE
    );
    loop {}
}

#[inline]
pub(crate) fn prepare_registers() {
    unsafe { sstatus::set_spp(sstatus::SPP::Supervisor) };
}

// 需保证该函数所有变量的生命周期在asm!()之前结束
#[no_mangle]
pub(crate) unsafe fn run_program(/*process: Process*/) {
    let mgr = PROCESS_MANAGER.get();
    let process = mgr.get_process();
    let kernel_ctx_ptr = core::ptr::addr_of!(mgr.kernel_ctx);
    let cloned = process.clone().unwrap();
    let guard = cloned.lock();
    let user_ctx_ptr = core::ptr::addr_of!(guard.process_ctx);
    drop(guard);
    drop(mgr);
    // trace!("arch_relate::run_program entered");
    arch_relate::timer::set_nxt_trigger();
    switch(kernel_ctx_ptr, user_ctx_ptr);
}

#[no_mangle]
#[naked]
pub(crate) unsafe extern "C" fn switch(from: *const ProcessContext, to: *const ProcessContext) {
    naked_asm!(
        save!(x1 => a0[0]),
        save!(x2 => a0[1]),
        save!(x8 => a0[2]),
        save!(x9 => a0[3]),
        save!(x18 => a0[4]),
        save!(x19 => a0[5]),
        save!(x20 => a0[6]),
        save!(x21 => a0[7]),
        save!(x22 => a0[8]),
        save!(x23 => a0[9]),
        save!(x24 => a0[10]),
        save!(x25 => a0[11]),
        save!(x26 => a0[12]),
        save!(x27 => a0[13]),
        load!(a1[0] => x1),
        load!(a1[1] => x2),
        load!(a1[2] => x8),
        load!(a1[3] => x9),
        load!(a1[4] => x18),
        load!(a1[5] => x19),
        load!(a1[6] => x20),
        load!(a1[7] => x21),
        load!(a1[8] => x22),
        load!(a1[9] => x23),
        load!(a1[10] => x24),
        load!(a1[11] => x25),
        load!(a1[12] => x26),
        load!(a1[13] => x27),
        "ret",
    )
}

pub(crate) fn enable_kernel_interrupt() {
    unsafe { riscv::register::sstatus::set_sie() };
}

pub(crate) fn disable_kernel_interrupt() {
    unsafe { riscv::register::sstatus::clear_sie() };
}

#[inline]
pub(crate) fn to_token(address: usize) -> usize {
    8usize << 60 | address
}

#[no_mangle]
pub(crate) fn enable_virtual_address(root_page_address: usize) {
    riscv::register::satp::write(to_token(root_page_address));
    unsafe { asm!("sfence.vma") }
}

#[allow(unused)]
macro_rules! fence {
    () => {
        core::arch::asm!("fence.i");
    };
}
