#[macro_use]
mod riscv64;

#[cfg(target_arch = "riscv64")]
pub(crate) use riscv64::*;
