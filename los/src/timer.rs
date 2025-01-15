use crate::arch_relate;

pub(crate) fn init() {
    trace!("Time Slice Interrupt init");
    arch_relate::timer::enable_interrupt();
}
