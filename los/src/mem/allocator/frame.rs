use core::ops::Deref;

use alloc::vec::Vec;
use spin::Mutex;

use crate::mem::address::PhyPageNum;

use super::FRAME_ALLOCATOR;

pub(crate) trait FrameAllocator {
    fn alloc(&mut self) -> Option<PhyPageNum>;
    fn alloc_fresh(&mut self) -> Option<PhyPageNum>;
    fn dealloc(&mut self, ppn: PhyPageNum);
}

pub(crate) struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    pub(crate) const fn const_new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    pub(crate) fn init(&mut self, start: PhyPageNum, end: PhyPageNum) {
        self.current = start.0;
        self.end = end.0;
    }
}

impl FrameAllocator for StackFrameAllocator {
    fn alloc(&mut self) -> Option<PhyPageNum> {
        if let Some(frame) = self.recycled.pop() {
            Some(frame.into())
        } else {
            self.alloc_fresh()
        }
    }

    #[inline]
    fn alloc_fresh(&mut self) -> Option<PhyPageNum> {
        if self.current < self.end {
            let res = self.current;
            self.current += 1;
            Some(res.into())
        } else {
            None
        }
    }

    fn dealloc(&mut self, ppn: PhyPageNum) {
        let ppn_num = ppn.0;
        if ppn_num < self.current
            && self
                .recycled
                .iter()
                .find(|item| **item == ppn_num)
                .is_none()
        {
            self.recycled.push(ppn_num);
        } else {
            panic!("frame illegal");
        }
    }
}

pub(crate) struct LockedStackFrameAllocator {
    inner: Mutex<StackFrameAllocator>,
}

impl LockedStackFrameAllocator {
    pub(crate) const fn const_new() -> Self {
        Self {
            inner: Mutex::new(StackFrameAllocator::const_new()),
        }
    }

    pub(crate) fn init(&self, start: PhyPageNum, end: PhyPageNum) {
        self.inner.lock().init(start, end);
    }

    pub(crate) fn alloc(&self) -> Option<FrameTracker> {
        self.inner.lock().alloc().map(FrameTracker::new)
    }

    pub(crate) fn alloc_fresh(&self) -> Option<FrameTracker> {
        self.inner.lock().alloc_fresh().map(FrameTracker::new)
    }

    pub(crate) fn dealloc(&self, ppn: PhyPageNum) {
        self.inner.lock().dealloc(ppn);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FrameTracker {
    pub(crate) ppn: PhyPageNum,
}

impl FrameTracker {
    pub(crate) fn new(ppn: PhyPageNum) -> Self {
        let bytes = ppn.get_bytes_array();
        for byte in bytes {
            *byte = 0;
        }
        Self { ppn }
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        FRAME_ALLOCATOR.dealloc(self.ppn);
    }
}
