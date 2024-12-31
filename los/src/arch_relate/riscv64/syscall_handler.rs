use riscv::register::{scause::{self, Exception, Trap}, sstatus};
use trap::{trap_restore, TrapContext};

use crate::{batch::{exit, restore_to_kernel, APP_MANAGER}, syscall::syscall};

#[macro_use]
pub(crate) mod trap;

#[no_mangle]
pub fn syscall_service(mut ctx: TrapContext) {
    // log!("syscall_service::user_sp: {:x}", ctx.info.sp);
    match scause::read().cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            ctx.ret_at_nxt();
            APP_MANAGER.get().mark_pc(ctx.spec);
            // ctx.set_syscall_res(syscall(ctx.get_syscall_id(), [ctx.get_args(0), ctx.get_args(1), ctx.get_args(2)]));
            match syscall(ctx.get_syscall_id(), [ctx.get_args(0), ctx.get_args(1), ctx.get_args(2)]) {
                crate::batch::RestoreBehavior::DirectReturen(res) => {
                    ctx.set_syscall_res(res);
                    unsafe {
                        trap_restore(&mut ctx);
                    };
                },
                crate::batch::RestoreBehavior::Reschedule => {
                    APP_MANAGER.get().mark_ctx(ctx);
                    restore_to_kernel();
                },
            }
        },
        Trap::Exception(e) => {
            if sstatus::read().spp() == sstatus::SPP::Supervisor {
                panic!("Kernel running into error: {:?}", e);
            }
            log!("{:?} in application, kernel killed it.", e);
            exit(255)
        }
        Trap::Interrupt(i) => {
            log!("Unsupported interrupt: {:?}, kernel kill it simply", i);
            exit(255)
        }
    }

}
