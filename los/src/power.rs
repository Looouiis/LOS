use crate::arch_relate::ecall::reset;

pub fn shutdown() -> ! {
    log!("LOS shutdown normally");
    reset(false);
}