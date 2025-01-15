#![allow(unused)]

use address::PhyAddr;
use allocator::{heap::HEAP, FRAME_ALLOCATOR, HEAP_ALLOCATOR};
use memory_set::KERNEL_SPACE;

use crate::config::{HEAP_SIZE, MEMORY_END};

pub(crate) mod address;
pub(crate) mod allocator;
pub(crate) mod memory_set;
pub(crate) mod page_table;

extern "C" {
    fn __kernel_end();
}

pub(crate) fn init() {
    HEAP_ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP_SIZE);
    trace!("Buddy System init");
    FRAME_ALLOCATOR.lock().init(
        PhyAddr::from(__kernel_end as usize).ceil_to_ppn(),
        PhyAddr::from(MEMORY_END).floor_to_ppn(),
    );
    trace!("Frame Allocator init");
    KERNEL_SPACE.get().activate();
    trace!("Kernel Space activated");
}
