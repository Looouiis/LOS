use alloc::vec::Vec;
use alloc::vec;

use super::{address::{PhyPageNum, VirPageNum}, allocator::{frame::FrameTracker, FRAME_ALLOCATOR}};

bitflags! {
    #[derive(PartialEq)]
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

#[derive(Copy, Clone)]
#[repr(C)]
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
        PhyPageNum::from(self.bits >> 10 & ((1usize << 44) - 1))
    }

    pub(crate) fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.bits & 1 == 1
    }

    pub(crate) fn clear_valid(&mut self) {
        self.bits &= !1;
    }

    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

#[derive(Clone)]
pub(crate) struct PageTable {
    root_ppn: PhyPageNum,
    frames: Vec<FrameTracker>
}

impl PageTable {
    pub(crate) fn new() -> Self {
        let frame = FRAME_ALLOCATOR.alloc().unwrap();
        Self {
            root_ppn: frame.ppn,
            frames: vec![frame]
        }
    }

    // frames 字段为空，也即不实际控制任何资源
    pub(crate) fn from_satp(satp: usize) -> Self {
        Self {
            root_ppn: (satp & ((1 << 44) - 1)).into(),
            frames: Vec::new(),
        }
    }

    pub(crate) fn address(&self) -> usize {
        self.root_ppn.into()
    }

    pub(crate) fn vpn_to_pte(&self, vpn: VirPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn)
            .map(|pte| pte.clone())
    }

    pub(crate) fn find_pte_create(&mut self, vpn: VirPageNum) -> Option<&mut PageTableEntry> {
        let indexs = vpn.get_index();
        let mut ppn = self.root_ppn;
        for (i, item) in indexs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*item];
            if i == 2 {
                return Some(pte);
            }
            if !pte.is_valid() {
                let frame = FRAME_ALLOCATOR.alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                assert!(pte.is_valid());
                self.frames.push(frame);
            }
            ppn = pte.ppn()
        }
        None
    }

    pub(crate) fn find_pte(&self, vpn: VirPageNum) -> Option<&mut PageTableEntry> {
        let indexs = vpn.get_index();
        let mut ppn = self.root_ppn;
        for (i, item) in indexs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*item];
            if i == 2 {
                return Some(pte);
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn()
        }
        None
    }

    pub(crate) fn build_reflect(&mut self, vpn: VirPageNum, ppn: PhyPageNum, flags: PTEFlags) {
        match self.find_pte_create(vpn) {
            Some(pte) => {
                assert!(!pte.is_valid(), "{vpn:?} has already mapped");
                *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
            },
            None => panic!("Failed to create pte"),
        }
    }

    pub(crate) fn remove_reflect(&self, vpn: VirPageNum) {
        match  self.find_pte(vpn) {
            Some(pte) => {
                assert!(pte.is_valid(), "{vpn:?} has not been mapped yet");
                // *pte = PageTableEntry::empty();
                pte.clear_valid();
                assert!(!pte.is_valid(), "internal error for pte.clear_valid()");
            },
            None => panic!("This Page Table item hasn't been create yet"),
        }
    }
}
