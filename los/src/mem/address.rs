use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};

const PHYSICAL_ADDRESS_WIDTH_SV39: usize = 56;

pub const PPN_WIDTH_SV39: usize = PHYSICAL_ADDRESS_WIDTH_SV39 - PAGE_SIZE_BITS;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct PhyAddr(usize);

impl From<usize> for PhyAddr {
    fn from(value: usize) -> Self {
        Self(value & ((1 << PHYSICAL_ADDRESS_WIDTH_SV39) - 1))
    }
}

impl From<PhyPageNum> for PhyAddr {
    fn from(value: PhyPageNum) -> Self {
        Self(value.0 << PAGE_SIZE_BITS)
    }
}

impl From<PhyAddr> for usize {
    fn from(value: PhyAddr) -> Self {
        value.0
    }
}

impl PhyAddr {
    pub(crate) fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }

    pub(crate) fn floor_to_ppn(&self) -> PhyPageNum {
        PhyPageNum(self.0 / PAGE_SIZE)
    }

    pub(crate) fn ceil_to_ppn(&self) -> PhyPageNum {
        PhyPageNum((self.0 + PAGE_SIZE - 1) / PAGE_SIZE)
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct VirAddr(usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct PhyPageNum(pub(crate) usize);

impl PhyPageNum {
    pub(crate) fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhyAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096) }
    }
}

impl From<usize> for PhyPageNum {
    fn from(value: usize) -> Self {
        Self(value & ((1 << PPN_WIDTH_SV39) - 1))
    }
}

impl From<PhyAddr> for PhyPageNum {
    fn from(value: PhyAddr) -> Self {
        assert_eq!(value.page_offset(), 0);
        value.floor_to_ppn()
    }
}

impl From<PhyPageNum> for usize {
    fn from(value: PhyPageNum) -> Self {
        value.0
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct VirPageNum(usize);