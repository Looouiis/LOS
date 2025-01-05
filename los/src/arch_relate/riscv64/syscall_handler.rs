use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sstatus,
};
use crate::{
    batch::{exit, reschedule, PROGRAM_MANAGER}, syscall::syscall, temp_test
};

use super::{timer::set_nxt_trigger, trap::{trap_restore, TrapContext}};

#[no_mangle]
pub fn syscall_service(mut ctx: TrapContext) {
    match scause::read().cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            ctx.ret_at_nxt();
            PROGRAM_MANAGER.get().mark_pc(ctx.spec);
            match syscall(
                ctx.get_syscall_id(),
                [ctx.get_args(0), ctx.get_args(1), ctx.get_args(2)],
            ) {
                crate::batch::RestoreBehavior::DirectReturen(res) => {
                    ctx.set_syscall_res(res);
                    unsafe {
                        trap_restore(&mut ctx);
                    };
                }
                crate::batch::RestoreBehavior::Reschedule => {
                    reschedule(ctx);
                }
            }
        }
        Trap::Exception(e) => {
            if sstatus::read().spp() == sstatus::SPP::Supervisor {
                panic!("Kernel running into error: {:?}", e);
            }
            log!("{:?} in program, kernel kill it.", e);
            exit(255)
        }
        Trap::Interrupt(i) => {
            if sstatus::read().spp() == sstatus::SPP::Supervisor {
                temp_test::trigger_kernel_interrupt();
                set_nxt_trigger();
            } else if Interrupt::SupervisorTimer == i {
                reschedule(ctx);
            } else {
                log!("Unsupported interrupt: {:?}, kernel kill it simply", i);
                exit(255)
            }
        }
    }
}
