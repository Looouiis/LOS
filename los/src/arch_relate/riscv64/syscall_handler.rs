use riscv::register::scause::{self, Exception, Trap};
use trap::{trap_restore, TrapContext};

use crate::{batch::restore_to_kernel, syscall::syscall};

#[macro_use]
pub(crate) mod trap;

#[no_mangle]
pub fn syscall_service(ctx: &mut TrapContext) {
    match scause::read().cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            ctx.info.a0 = syscall(ctx.info.a7, [ctx.info.a0, ctx.info.a1, ctx.info.a2]);
            ctx.spec += 4;
            unsafe {
                trap_restore(ctx);
            };
        },
        Trap::Exception(e) => {
            log!("{:?} in application, kernel killed it.", e);
            restore_to_kernel();
        }
        Trap::Interrupt(i) => {
            log!("Unsupported interrupt: {:?}, kernel kill it simply", i);
            restore_to_kernel();
        }
    }

}
