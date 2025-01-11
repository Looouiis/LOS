use alloc::vec::Vec;
use alloc::vec;

use super::{address::{PhyPageNum, VirPageNum}, allocator::{frame::FrameTracker, FRAME_ALLOCATOR}};

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

pub(crate) struct PageTableEntry {
    pub(crate) bits: usize,
}

impl PageTableEntry {
    pub(crate) fn new(ppn: PhyPageNum, flags: PTEFlags) -> Self {
        Self {
            bits: ppn.0 << 10 | flags.bits() as usize
        }
    }

    pub(crate) fn empty() -> Self {
        Self {
            bits: 0
        }
    }

    pub(crate) fn ppn(&self) -> PhyPageNum {
        PhyPageNum(self.bits >> 10 & ((1usize << 44) - 1))
    }

    pub(crate) fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.bits & 1 == 1
    }
}

pub(crate) struct PageTable {
    root_ppn: PhyPageNum,
    frames: Vec<FrameTracker>
}

impl PageTable {
    pub fn new() -> Self {
        let frame = FRAME_ALLOCATOR.alloc().unwrap();
        Self {
            root_ppn: frame.ppn,
            frames: vec![frame]
        }
    }
}
