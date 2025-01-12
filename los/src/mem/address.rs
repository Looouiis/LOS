use core::fmt::Debug;

use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};

use super::page_table::PageTableEntry;

const PHYSICAL_ADDRESS_WIDTH_SV39: usize = 56;

const VIRTUAL_ADDRESS_WIDTH_SV39: usize = 39;

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
pub(crate) struct VirAddr(pub(crate) usize);

impl VirAddr {
    pub(crate) fn floor_to_vpn(&self) -> VirPageNum {
        VirPageNum(self.0 / PAGE_SIZE)
    }

    pub(crate) fn ceil_to_vpn(&self) -> VirPageNum {
        if self.0 == 0 {
            VirPageNum(0)
        }
        else {
            VirPageNum((self.0 + PAGE_SIZE - 1) / PAGE_SIZE)
        }
    }
}

impl From<usize> for VirAddr {
    fn from(value: usize) -> Self {
        Self(value & ((1 << VIRTUAL_ADDRESS_WIDTH_SV39) - 1))
    }
}

impl From<VirPageNum> for VirAddr {
    fn from(value: VirPageNum) -> Self {
        Self(value.0 << PAGE_SIZE_BITS)
    }
}

impl From<VirAddr> for usize {
    fn from(value: VirAddr) -> Self {
        if value.0 >= (1 << (VIRTUAL_ADDRESS_WIDTH_SV39 - 1)) {     // 高256GB情况：将高位置一
            value.0 | (!((1 << VIRTUAL_ADDRESS_WIDTH_SV39) - 1))
        }
        else {
            value.0
        }
    }
}

impl Debug for VirAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("va:{:#x}", self.0))
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct PhyPageNum(pub(crate) usize);

// "开启之后，虽然裸指针被视为一个虚拟地址，但是上面已经提到，基于恒等映射，虚拟地址会映射到一个相同的物理地址，因此在也是成立的"
impl<'a> PhyPageNum {
    pub(crate) const fn empty() -> Self {
        Self(0)
    }

    pub(crate) fn get_pte_array(&self) -> &'a mut [PageTableEntry] {
        let pa: PhyAddr = (*self).into();
        unsafe {
            core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512)
        }
    }

    pub(crate) fn get_bytes_array(&self) -> &'a mut [u8] {
        let pa: PhyAddr = (*self).into();
        unsafe {
            core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096)
        }
    }

    pub(crate) fn get_mut_data_at_start<T>(&self) -> &'a mut T {
        let pa: PhyAddr = (*self).into();
        unsafe {
            (pa.0 as *mut T).as_mut().unwrap()
        }
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
pub(crate) struct VirPageNum(pub(crate) usize);

impl VirPageNum {
    // 0为第一级，1为第二级，2为第三级
    pub(crate) fn get_index(&self) -> [usize; 3] {
        let mut idx = [0; 3];
        let mut vpn = self.0;
        for i in (0 .. 3).rev() {
            idx[i] = vpn & ((1 << 9) - 1);
            vpn >>= 9;
        }
        idx
    }
}

impl From<VirAddr> for VirPageNum {
    fn from(value: VirAddr) -> Self {
        value.floor_to_vpn()
    }
}

impl Debug for VirPageNum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("vpn: {:#x}", self.0))
    }
}

pub(crate) trait StepByOne {
    fn step(&mut self);
}

impl StepByOne for VirPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}

#[derive(Copy, Clone)]
pub(crate) struct SimpleRange<T>
where T: StepByOne + Copy + Clone + Ord + PartialOrd + Eq + PartialEq + Debug
{
    start: T,
    end: T,
}

impl<T> SimpleRange<T>
where T: StepByOne + Copy + Clone + Ord + PartialOrd + Eq + PartialEq + Debug
{
    pub(crate) fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self {
            start,
            end
        }
    }

    pub(crate) fn get_start(&self) -> T {
        self.start
    }

    pub(crate) fn get_end(&self) -> T {
        self.end
    }
}

impl<T> IntoIterator for SimpleRange<T>
where T: StepByOne + Copy + Clone + Ord + PartialOrd + Eq + PartialEq + Debug
{
    type Item = T;

    type IntoIter = SimpleRangeIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.start, self.end)
    }
}

pub(crate) struct SimpleRangeIterator<T>
where T: StepByOne + Copy + Clone + Ord + PartialOrd + Eq + PartialEq + Debug
{
    current: T,
    end: T
}

impl<T> SimpleRangeIterator<T>
where T: StepByOne + Copy + Clone + Ord + PartialOrd + Eq + PartialEq + Debug
{
    fn new(current: T, end: T) -> Self {
        Self {
            current,
            end
        }
    }
}

impl<T> Iterator for SimpleRangeIterator<T>
where T: StepByOne + Copy + Clone + Ord + PartialOrd + Eq + PartialEq + Debug
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.end {
            let res = self.current;
            self.current.step();
            Some(res)
        } else {
            None
        }
    }
}

pub(crate) type VPNRange = SimpleRange<VirPageNum>;
