pub(crate) const KERNAL_STACK_SIZE: usize = 4096 * 2;

pub(crate) const USER_STACK_SIZE: usize = 4096 * 2;

#[repr(C, align(4096))]
pub(crate) struct KernelStack([u8; KERNAL_STACK_SIZE]);

#[link_section = ".bss.stack"]
pub(crate) static KERNAL_STACK: KernelStack = KernelStack([0; KERNAL_STACK_SIZE]);

#[link_section = ".bss.stack"]
pub(crate) static TRAP_STACK: KernelStack = KernelStack([0; KERNAL_STACK_SIZE]);
