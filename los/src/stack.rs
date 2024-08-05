// use core::arch::asm;

// const STACK_CNT: usize = 1;
// const SINGLE_STACK_SIZE: usize = 4096 * 16;

// #[link_section = ".data.stack"]
// pub(crate) static STACK_BOTTOM: [BootStack; STACK_CNT] = [BootStack::ZERO; STACK_CNT];
// pub(crate) const STACK_SIZE: usize = STACK_CNT * SINGLE_STACK_SIZE;

// #[repr(C, align(128))]
// pub(crate) struct BootStack([u8; SINGLE_STACK_SIZE]);

// impl BootStack {
//     const ZERO: Self = Self([0; SINGLE_STACK_SIZE]);
// }

pub(crate) const KERNAL_STACK_SIZE: usize = 4096 * 2;

const USER_STACK_SIZE: usize = 4096 * 2;

#[repr(C, align(4096))]
pub(crate) struct KernelStack([u8; KERNAL_STACK_SIZE]);

impl KernelStack {
    // pub(crate) fn get_sp_top(&self) -> usize {
    //     self.0.as_ptr() as usize + KERNAL_STACK_SIZE
    // }
}

#[repr(C, align(4096))]
pub(crate) struct UserStack([u8; USER_STACK_SIZE]);

impl UserStack {
    pub(crate) fn get_sp_top(&self) -> usize {
        self.0.as_ptr() as usize + USER_STACK_SIZE
    }
}

#[link_section = ".bss.stack"]
pub(crate) static KERNAL_STACK: KernelStack = KernelStack([0; KERNAL_STACK_SIZE]);

pub(crate) static USER_STACK: UserStack = UserStack([0; USER_STACK_SIZE]);
