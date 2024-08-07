use riscv::register::{scause::{self, Exception, Trap}, sstatus};
use trap::{trap_restore, TrapContext};

use crate::{batch::restore_to_kernel, syscall::syscall};

#[macro_use]
pub(crate) mod trap;

#[no_mangle]
pub fn syscall_service(ctx: &mut TrapContext) {
    match scause::read().cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            ctx.set_syscall_res(syscall(ctx.get_syscall_id(), [ctx.get_args(0), ctx.get_args(1), ctx.get_args(2)]));
            ctx.ret_at_nxt();
            unsafe {
                trap_restore(ctx);
            };
        },
        Trap::Exception(e) => {
            // if batch::RUNNING.load(core::sync::atomic::Ordering::Relaxed) {
            if sstatus::read().spp() == sstatus::SPP::Supervisor {
                panic!("Kernel running into error: {:?}", e);
            }
            log!("{:?} in application, kernel killed it.", e);
            restore_to_kernel();
        }
        Trap::Interrupt(i) => {
            log!("Unsupported interrupt: {:?}, kernel kill it simply", i);
            restore_to_kernel();
        }
    }

}
