use heap::{BuddyAllocator, HEAP};

use crate::config::HEAP_SIZE;

pub(crate) mod heap;

#[global_allocator]
pub(crate) static HEAP_ALLOCATOR: BuddyAllocator = BuddyAllocator::uninit();

pub(crate) fn init() {
        HEAP_ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP_SIZE);
}
