use riscv::register::time;

use crate::config::CLOCK_FREQ;

const TICKS_PER_SEC: usize = 100;
// const MICRO_PER_SEC: usize = 1000000;

pub(crate) fn get_time() -> usize {
    time::read()
}

pub(crate) fn set_nxt_trigger() {
    sbi_rt::set_timer((get_time() + CLOCK_FREQ / TICKS_PER_SEC) as u64);
}

pub(crate) fn enable_interrupt() {
    unsafe { riscv::register::sie::set_stimer() };
}

// pub(crate) fn get_time_us() -> usize {
//     get_time() / (CLOCK_FREQ / MICRO_PER_SEC)
// }
