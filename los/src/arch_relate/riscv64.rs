use core::arch::asm;
use riscv::register::sstatus;
use trap::{get_restore_va, TrapContext};

use crate::{batch::PROGRAM_MANAGER, config::{TRAMPOLINE, TRAP_CONTEXT}, stack};

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
            la      t0, {trap_vec}
            csrw    stvec, t0
            j       rust_main
        ",
        stack_btn = sym KERNAL_STACK,
        stack_size = const KERNAL_STACK_SIZE,
        // trap_vec = sym trap::trap_vec
        trap_vec = const TRAMPOLINE
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
    const CLEAR_SPP: usize = !(1usize << 8);
    sstatus::set_spp(sstatus::SPP::Supervisor);
    assert!(sstatus::read().spp() == sstatus::SPP::Supervisor);
    let mgr = PROGRAM_MANAGER.get();
    let guard = mgr.get_process().lock();
    let satp = match guard.as_ref() {
        Some(process) => process.get_satp(),
        None => return,
    };
    let kernel_ctx_ptr = core::ptr::addr_of!(mgr.kernel_ctx);
    // let ctx = process.get_trap_context();
    // let entry = ctx.sepc;
    // let satp = process.get_satp();
    let restore_va = get_restore_va();
    drop(guard);
    drop(mgr);
    trace!("arch_relate::run_program entered");
    asm!(
        // save!(x1 => a3[1]),
        save!(x2 => a3[2]),
        // save!(x3 => a3[3]),
        // save!(x5 => a3[5]),
        // save!(x6 => a3[6]),
        // save!(x7 => a3[7]),
        save!(x8 => a3[8]),
        save!(x9 => a3[9]),
        // save!(x10 => a3[10]),
        // save!(x11 => a3[11]),
        // save!(x12 => a3[12]),
        // save!(x13 => a3[13]),
        // save!(x14 => a3[14]),
        // save!(x15 => a3[15]),
        // save!(x16 => a3[16]),
        // save!(x17 => a3[17]),
        save!(x18 => a3[18]),
        save!(x19 => a3[19]),
        save!(x20 => a3[20]),
        save!(x21 => a3[21]),
        save!(x22 => a3[22]),
        save!(x23 => a3[23]),
        save!(x24 => a3[24]),
        save!(x25 => a3[25]),
        save!(x26 => a3[26]),
        save!(x27 => a3[27]),
        // save!(x28 => a3[28]),
        // save!(x29 => a3[29]),
        // save!(x30 => a3[30]),
        // save!(x31 => a3[31]),
        "   csrr t3, sstatus
            csrw sepc, t0
            csrw sscratch, a0
            la t2, 0f
            addi t2, t2, 4
        ",
        save!(t2 => a3[33]),
        save!(t3 => a3[32]),
        "   csrr a3, sstatus
            andi a3, a3, {clear_spp}
            csrw sstatus, a3

            fence.i
        0:
            jr {restore_va}

            // csrw satp, t1
            // sfence.vma",
        // // load!(a0[1] => x1),
        // load!(a0[2] => x2),
        // // load!(a0[3] => x3),
        // // load!(a0[5] => x5),
        // // load!(a0[6] => x6),
        // // load!(a0[7] => x7),
        // load!(a0[8] => x8),
        // load!(a0[9] => x9),
        // // load!(a0[10] => x10),
        // // load!(a0[11] => x11),
        // // load!(a0[13] => x13),
        // // load!(a0[14] => x14),
        // // load!(a0[15] => x15),
        // // load!(a0[16] => x16),
        // // load!(a0[17] => x17),
        // load!(a0[18] => x18),
        // load!(a0[19] => x19),
        // load!(a0[20] => x20),
        // load!(a0[21] => x21),
        // load!(a0[22] => x22),
        // load!(a0[23] => x23),
        // load!(a0[24] => x24),
        // load!(a0[25] => x25),
        // load!(a0[26] => x26),
        // load!(a0[27] => x27),
        // // load!(a0[28] => x28),
        // // load!(a0[29] => x29),
        // // load!(a0[30] => x30),
        // // load!(a0[31] => x31),
        // // load!(a0[12] => x12),
        "0:
            sret
            ",

        // in("t0") entry,
        in("a0") TRAP_CONTEXT,
        in("a3") kernel_ctx_ptr,
        in("a1") satp,
        restore_va = in(reg) restore_va,
        clear_spp = const CLEAR_SPP,
    );
}

pub(crate) fn enable_kernel_interrupt() {
    unsafe { riscv::register::sstatus::set_sie() };
}

pub(crate) fn disable_kernel_interrupt() {
    unsafe { riscv::register::sstatus::clear_sie() };
}

#[inline]
pub(crate) fn to_satp(address: usize) -> usize {
    8usize << 60 | address
}

#[no_mangle]
pub(crate) fn enable_virtual_address(root_page_address: usize) {
    riscv::register::satp::write(to_satp(root_page_address));
    unsafe {
        asm!("sfence.vma")
    }
}

#[allow(unused)]
macro_rules! fence {
    () => {
        core::arch::asm!("fence.i");
    };
}
