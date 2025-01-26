#[allow(deprecated)]
pub fn putch(c: usize) {
    sbi_rt::legacy::console_putchar(c);
}

#[allow(deprecated)]
pub fn getch() -> usize {
    sbi_rt::legacy::console_getchar()
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
