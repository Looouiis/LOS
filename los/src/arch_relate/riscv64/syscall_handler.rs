use crate::{
    arch_relate::trap::trap_return,
    process::{reschedule, restore_to_kernel, syscall_fn::exit},
    syscall::syscall,
};
use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sstatus,
};

use super::{timer::set_nxt_trigger, PROGRAM_MANAGER};

#[no_mangle]
pub fn syscall_service() {
    let ctx = PROGRAM_MANAGER.get().get_current_trap_context();
    match scause::read().cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            ctx.ret_at_nxt();
            match syscall(
                ctx.get_syscall_id(),
                [ctx.get_args(0), ctx.get_args(1), ctx.get_args(2)],
            ) {
                crate::process::RestoreBehavior::DirectReturn(res) => {
                    ctx.set_syscall_res(res);
                    // trap_return(false);
                    trap_return();
                }
                crate::process::RestoreBehavior::Reschedule => {
                    reschedule();
                }
            }
        }
        Trap::Exception(e) => {
            let sepc = riscv::register::sepc::read();
            if sstatus::read().spp() == sstatus::SPP::Supervisor {
                panic!("Kernel running into error: {:?}, sepc: {sepc:x}", e);
            }
            log!("{:?} in program, kernel kill it.", e);
            exit(255)
        }
        Trap::Interrupt(i) => {
            if sstatus::read().spp() == sstatus::SPP::Supervisor {
                crate::temp_test::trigger_kernel_interrupt();
                set_nxt_trigger();
                restore_to_kernel();
            } else if Interrupt::SupervisorTimer == i {
                reschedule();
            } else {
                log!("Unsupported interrupt: {:?}, kernel kill it simply", i);
                exit(255)
            }
        }
    }
    trap_return();
}
