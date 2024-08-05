use core::arch::asm;

use riscv::register::sstatus::{self, Sstatus};

#[cfg(target_pointer_width = "32")]
#[macro_use]
mod arch {
    pub(crate) const WORD_SIZE: usize = 4;
    macro_rules! save {
        ($reg:ident => $ptr:ident[$pos:expr]) => {
            concat!(
                "sw ",
                stringify!($reg),
                ", 4*",
                $pos,
                '(',
                stringify!($ptr),
                ')'
            )
        };
    }

    macro_rules! load {
        ($ptr:ident[$pos:expr] => $reg:ident) => {
            concat!(
                "lw ",
                stringify!($reg),
                ", 4*",
                $pos,
                '(',
                stringify!($ptr),
                ')'
            )
        };
    }
}

#[cfg(target_pointer_width = "64")]
#[macro_use]
mod arch {
    pub(crate) const WORD_SIZE: usize = 8;
    macro_rules! save {
        ($reg:ident => $ptr:ident[$pos:expr]) => {
            concat!(
                "sd ",
                stringify!($reg),
                ", 8*",
                $pos,
                '(',
                stringify!($ptr),
                ')'
            )
        };
    }

    macro_rules! load {
        ($ptr:ident[$pos:expr] => $reg:ident) => {
            concat!(
                "ld ",
                stringify!($reg),
                ", 8*",
                $pos,
                '(',
                stringify!($ptr),
                ')'
            )
        };
    }
}

#[no_mangle]
pub(crate) unsafe extern "C" fn trap_vec() {
    asm!(
        "   .align 4
            .option push
            .option norvc
            j {handler}
            .option pop
        ",
        handler = sym trap_handler,
        options(noreturn)
    )
}

#[no_mangle]
pub(crate) unsafe extern "C" fn trap_handler() {
    asm!(
        "   csrrw  sp, sscratch, sp
            addi sp, sp, -34 * {word_size}
        ",
        save!(x1 => sp[1]),
        save!(x3 => sp[3]),
        save!(x5 => sp[5]),
        save!(x6 => sp[6]),
        save!(x7 => sp[7]),
        save!(x8 => sp[8]),
        save!(x9 => sp[9]),
        save!(x10 => sp[10]),
        save!(x11 => sp[11]),
        save!(x12 => sp[12]),
        save!(x13 => sp[13]),
        save!(x14 => sp[14]),
        save!(x15 => sp[15]),
        save!(x16 => sp[16]),
        save!(x17 => sp[17]),
        save!(x18 => sp[18]),
        save!(x19 => sp[19]),
        save!(x20 => sp[20]),
        save!(x21 => sp[21]),
        save!(x22 => sp[22]),
        save!(x23 => sp[23]),
        save!(x24 => sp[24]),
        save!(x25 => sp[25]),
        save!(x26 => sp[26]),
        save!(x27 => sp[27]),
        save!(x28 => sp[28]),
        save!(x29 => sp[29]),
        save!(x30 => sp[30]),
        save!(x31 => sp[31]),
        "   csrr t0, sstatus
            csrr t1, sepc
            csrr t2, sscratch
        ",
        save!(t0 => sp[32]),
        save!(t1 => sp[33]),
        save!(t2 => sp[2]),
        "   mv a0, sp
            j syscall_service
        ",
        word_size = const arch::WORD_SIZE,
        options(noreturn)
    );
}

#[no_mangle]
pub(crate) unsafe extern "C" fn trap_restore(ctx: &mut TrapContext) -> ! {
    asm!(
        "   mv sp, a0",
        load!(sp[32] => t0),
        load!(sp[33] => t1),
        load!(sp[2] => t2),
        "   csrw sstatus, t0
            csrw sepc, t1
            csrw sscratch, t2
        ",
        load!(sp[1] => x1),
        load!(sp[3] => x3),
        load!(sp[5] => x5),
        load!(sp[6] => x6),
        load!(sp[7] => x7),
        load!(sp[8] => x8),
        load!(sp[9] => x9),
        load!(sp[10] => x10),
        load!(sp[11] => x11),
        load!(sp[12] => x12),
        load!(sp[13] => x13),
        load!(sp[14] => x14),
        load!(sp[15] => x15),
        load!(sp[16] => x16),
        load!(sp[17] => x17),
        load!(sp[18] => x18),
        load!(sp[19] => x19),
        load!(sp[20] => x20),
        load!(sp[21] => x21),
        load!(sp[22] => x22),
        load!(sp[23] => x23),
        load!(sp[24] => x24),
        load!(sp[25] => x25),
        load!(sp[26] => x26),
        load!(sp[27] => x27),
        load!(sp[28] => x28),
        load!(sp[29] => x29),
        load!(sp[30] => x30),
        load!(sp[31] => x31),
        "
            addi sp, sp, 34 * {word_size}
            csrrw  sp, sscratch, sp
            sret
        ",
        in ("a0") ctx,
        word_size = const arch::WORD_SIZE,
        options(noreturn)
    );
}

#[repr(C)]
pub struct TrapContext {
    pub info: RegInfo,
    pub sstatus: Sstatus,
    pub spec: usize,
}

impl TrapContext {
    pub(crate) fn new() -> Self {
        TrapContext {
            info: RegInfo::new(),
            sstatus: sstatus::read(),
            spec: 0
        }
    }
}

#[allow(unused)]
pub(crate) struct RegInfo {
    pub(crate) x0: usize,
    pub(crate) ra: usize,
    pub(crate) sp: usize,
    pub(crate) gp: usize,
    pub(crate) tp: usize,
    pub(crate) t0: usize,
    pub(crate) t1: usize,
    pub(crate) t2: usize,
    pub(crate) fp: usize,
    pub(crate) s1: usize,
    pub(crate) a0: usize,
    pub(crate) a1: usize,
    pub(crate) a2: usize,
    pub(crate) a3: usize,
    pub(crate) a4: usize,
    pub(crate) a5: usize,
    pub(crate) a6: usize,
    pub(crate) a7: usize,
    pub(crate) s2: usize,
    pub(crate) s3: usize,
    pub(crate) s4: usize,
    pub(crate) s5: usize,
    pub(crate) s6: usize,
    pub(crate) s7: usize,
    pub(crate) s8: usize,
    pub(crate) s9: usize,
    pub(crate) s10: usize,
    pub(crate) s11: usize,
    pub(crate) t3: usize,
    pub(crate) t4: usize,
    pub(crate) t5: usize,
    pub(crate) t6: usize
}

impl RegInfo {
    fn new() -> Self {
        Self {
            x0: 0,
            ra: 0,
            sp: 0,
            gp: 0,
            tp: 0,
            t0: 0,
            t1: 0,
            t2: 0,
            fp: 0,
            s1: 0,
            a0: 0,
            a1: 0,
            a2: 0,
            a3: 0,
            a4: 0,
            a5: 0,
            a6: 0,
            a7: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
            t3: 0,
            t4: 0,
            t5: 0,
            t6: 0
        }
    }
}
