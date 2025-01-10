use heap::{BuddyAllocator, HEAP};

use crate::config::HEAP_SIZE;

pub(crate) mod heap;

#[global_allocator]
pub(crate) static HEAP_ALLOCATOR: BuddyAllocator = BuddyAllocator::uninit();

#[alloc_error_handler]
pub(crate) fn on_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap alloc error, layout = {:?}", layout)
}

pub(crate) fn init() {
    trace!("Buddy System init");
    HEAP_ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP_SIZE);
}
