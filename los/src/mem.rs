use address::PhyAddr;
use allocator::{heap::HEAP, FRAME_ALLOCATOR, HEAP_ALLOCATOR};

use crate::config::{HEAP_SIZE, MEMORY_END};

pub(crate) mod allocator;
pub(crate) mod address;
pub(crate) mod page_table;

pub(crate) fn init() {
    trace!("Buddy System init");
    HEAP_ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP_SIZE);
    trace!("Frame Allocator init");
    extern "C" {
        fn __kernel_end();
    }
    FRAME_ALLOCATOR
        .lock()   
        .init(PhyAddr::from(__kernel_end as usize).ceil_to_ppn(), PhyAddr::from(MEMORY_END).floor_to_ppn());
    FRAME_ALLOCATOR.alloc().unwrap();
}
