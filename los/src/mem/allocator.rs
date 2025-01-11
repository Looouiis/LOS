use frame::LockedStackFrameAllocator;
use heap::BuddyAllocator;


pub(crate) mod heap;
pub(crate) mod frame;

#[global_allocator]
pub(crate) static HEAP_ALLOCATOR: BuddyAllocator = BuddyAllocator::uninit();

pub(crate) static FRAME_ALLOCATOR: LockedStackFrameAllocator = LockedStackFrameAllocator::const_new();

#[alloc_error_handler]
pub(crate) fn on_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap alloc error, layout = {:?}", layout)
}
