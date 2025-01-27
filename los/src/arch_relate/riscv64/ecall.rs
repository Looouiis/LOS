use sbi_rt::Physical;

#[allow(deprecated)]
pub fn putch(c: usize) {
    sbi_rt::legacy::console_putchar(c);
}

pub fn getch() -> u8 {
    let ch = [0u8; 1];
    let ptr = &ch as *const u8 as usize;
    sbi_rt::console_read(Physical::new(1, ptr, ptr + 1));
    ch[0]
}

pub fn reset(failure: bool) -> ! {
    use sbi_rt::{system_reset, NoReason, Shutdown, SystemFailure};
    if !failure {
        system_reset(Shutdown, NoReason);
    } else {
        system_reset(Shutdown, SystemFailure);
    }
    unreachable!();
}
