use crate::arch_relate;

pub(crate) fn init() {
    arch_relate::timer::enable_interrupt();
    arch_relate::timer::set_nxt_trigger();
}
