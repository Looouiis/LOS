use core::arch::asm;
use riscv::register::sstatus;
use syscall_handler::trap::TrapContext;

use crate::{batch::{Process, APP_MANAGER}, stack::{self, USER_STACK}};

pub(crate) mod ecall;
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
        trap_vec = sym syscall_handler::trap::trap_vec
    );
    loop {}
}

#[inline]
pub(crate) fn prepare_registers() {
    unsafe { sstatus::set_spp(sstatus::SPP::Supervisor) };
}

// 需保证该函数所有变量的生命周期在asm!()之前结束
#[no_mangle]
pub(crate) unsafe fn run_app(process: Process/*user_top: usize, entry: usize*/) {
    let entry = process.pc;
    // log!("entry: {:x}", entry);
    const CLEAR_SPP: usize = !(1usize << 8);
    sstatus::set_spp(sstatus::SPP::Supervisor);
    assert!(sstatus::read().spp() == sstatus::SPP::Supervisor);
    let mgr = APP_MANAGER.get();
    let kernel_ctx_ptr = core::ptr::addr_of!(mgr.kernel_ctx);
    let user_ctx_ptr: *const TrapContext = core::ptr::addr_of!(process.ctx);
    // log!("run_app::user_sp: {:x}", process.ctx.info.sp);
    drop(mgr);
    trace!("arch_relate::run_app entered");
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
            csrrw sp, sscratch, sp
            la t1, 0f
            addi t1, t1, 4
        ",
        save!(t1 => a3[33]),
        save!(t3 => a3[32]),
        // load!(a2[1] => x1),
        load!(a2[2] => x2),
        // load!(a2[3] => x3),
        // load!(a2[5] => x5),
        // load!(a2[6] => x6),
        // load!(a2[7] => x7),
        load!(a2[8] => x8),
        load!(a2[9] => x9),
        // load!(a2[10] => x10),
        // load!(a2[11] => x11),
        // load!(a2[13] => x13),
        // load!(a2[14] => x14),
        // load!(a2[15] => x15),
        // load!(a2[16] => x16),
        // load!(a2[17] => x17),
        load!(a2[18] => x18),
        load!(a2[19] => x19),
        load!(a2[20] => x20),
        load!(a2[21] => x21),
        load!(a2[22] => x22),
        load!(a2[23] => x23),
        load!(a2[24] => x24),
        load!(a2[25] => x25),
        load!(a2[26] => x26),
        load!(a2[27] => x27),
        // load!(a2[28] => x28),
        // load!(a2[29] => x29),
        // load!(a2[30] => x30),
        // load!(a2[31] => x31),
        // load!(a2[12] => x12),
        "
            csrr a3, sstatus
            andi a3, a3, {clear_spp}
            csrw sstatus, a3
        0:
            sret
            ",

        in("t0") entry,
        in("a2") user_ctx_ptr,
        in("a3") kernel_ctx_ptr,
        clear_spp = const CLEAR_SPP,
    );
}

macro_rules! fence {
    () => {
        core::arch::asm!("fence.i");
    };
}