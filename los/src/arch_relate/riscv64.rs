use core::arch::asm;
use riscv::register::sstatus;

use crate::{batch::APP_MANAGER, stack};

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

#[inline]
pub(crate) unsafe fn run_app(user_top: usize) -> usize {
    const CLEAR_SPP: usize = !(1usize << 8);
    // asm!(
    //     "   csrw sepc, t0
    //         csrr t0, sstatus
    //         andi t0, t0, {clear_spp}
    //         csrw sstatus, t0
    //         csrw sscratch, t2
    //         li fp, 0        # 防止栈跟踪
    //         li ra, 0
    //         mv sp, t1
    //         sret
    //     ",
    //     in("t0") crate::batch::AppManager::ENTRY,
    //     in("t1") user_top,
    //     in("t2") kernel_top,
    //     clear_spp = const CLEAR_SPP,
    //     options(noreturn)
    // );
    // sstatus::set_spp(sstatus::SPP::Supervisor);
    unsafe {
        asm!(
            "   csrr t0, sstatus
                ori t0, t0, 1 << 8
                csrw sstatus, t0
            ",
        );
    }
    assert!(sstatus::read().spp() == sstatus::SPP::Supervisor);
    let mgr = APP_MANAGER.get();
    let ctx_ptr = core::ptr::addr_of!(mgr.kernel_ctx);
    drop(mgr);
    let res;
    asm!(
        // "la a3, {ctx}",
        save!(x1 => a3[1]),
        save!(x2 => a3[2]),
        save!(x3 => a3[3]),
        save!(x5 => a3[5]),
        save!(x6 => a3[6]),
        save!(x7 => a3[7]),
        save!(x8 => a3[8]),
        save!(x9 => a3[9]),
        save!(x10 => a3[10]),
        save!(x11 => a3[11]),
        save!(x12 => a3[12]),
        save!(x13 => a3[13]),
        save!(x14 => a3[14]),
        save!(x15 => a3[15]),
        save!(x16 => a3[16]),
        save!(x17 => a3[17]),
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
        save!(x28 => a3[28]),
        save!(x29 => a3[29]),
        save!(x30 => a3[30]),
        save!(x31 => a3[31]),
        "   csrr t3, sstatus
            csrw sepc, t0
            csrrw sp, sscratch, sp
            mv sp, t1
            la t1, 0f
            addi t1, t1, 4
        ",
        save!(t1 => a3[33]),
        save!(t3 => a3[32]),
        "   mv x1, x0
            mv x3, x0
            mv x5, x0
            mv x6, x0
            mv x7, x0
            mv x8, x0
            mv x9, x0
            mv x10, x0
            mv x11, x0
            mv x12, x0
            mv x13, x0
            mv x14, x0
            mv x15, x0
            mv x16, x0
            mv x17, x0
            mv x18, x0
            mv x19, x0
            mv x20, x0
            mv x21, x0
            mv x22, x0
            mv x23, x0
            mv x24, x0
            mv x25, x0
            mv x26, x0
            mv x27, x0
            mv x28, x0
            mv x29, x0
            mv x30, x0
            mv x31, x0
            csrr t0, sstatus
            andi t0, t0, {clear_spp}
            csrw sstatus, t0
        0:
            sret
            ",

        in("t1") user_top,
        in("t0") crate::batch::AppManager::ENTRY,
        in("a3") ctx_ptr,
        // ctx = sym mgr.kernel_ctx,
        clear_spp = const CLEAR_SPP,
    );
    trace!("arch_relate::run_app trace");
    asm!("", out ("a0") res);
    res
}

macro_rules! fence {
    () => {
        core::arch::asm!("fence.i");
    };
}