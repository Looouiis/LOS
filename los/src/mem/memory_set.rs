use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec::Vec};
use lazy_static::lazy_static;

use crate::{
    arch_relate,
    config::{MEMORY_END, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT},
    mem::PhyAddr,
    process::ArcCell,
    stack::USER_STACK_SIZE,
};

use super::{
    address::{PhyPageNum, StepByOne, VPNRange, VirAddr, VirPageNum},
    allocator::frame::FrameTracker,
    page_table::{PTEFlags, PageTable},
    FRAME_ALLOCATOR,
};

lazy_static! {
    pub(crate) static ref KERNEL_SPACE: Arc<ArcCell<MemorySet>> =
        Arc::new(ArcCell::new(MemorySet::build_kernel_space()));
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum MapType {
    Identical, // 恒等映射
    Framed,
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

pub(crate) struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirPageNum, FrameTracker>,
    map_type: MapType,
    map_per: MapPermission,
}

impl MapArea {
    pub(crate) fn new(
        start: VirAddr,
        end: VirAddr,
        map_type: MapType,
        map_per: MapPermission,
    ) -> Self {
        Self {
            vpn_range: VPNRange::new(start.floor_to_vpn(), end.ceil_to_vpn()),
            data_frames: BTreeMap::new(),
            map_type,
            map_per,
        }
    }

    fn reflect_one_vpn_to_page_table(&mut self, vpn: VirPageNum, page_table: &mut PageTable) {
        let ppn: PhyPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhyPageNum(vpn.0);
            }
            MapType::Framed => {
                let frame = FRAME_ALLOCATOR.alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }
        page_table.build_reflect(vpn, ppn, PTEFlags::from_bits(self.map_per.bits()).unwrap());
    }

    pub(crate) fn reflect_self_to_page_table(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.reflect_one_vpn_to_page_table(vpn, page_table);
        }
    }

    fn remove_one_vpn_from_page_table(&mut self, vpn: VirPageNum, page_table: &mut PageTable) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.remove_reflect(vpn);
    }

    pub(crate) fn remove_self_from_page_table(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.remove_one_vpn_from_page_table(vpn, page_table);
        }
    }

    pub(crate) fn copy_data(&self, page_table: &PageTable, data: &[u8]) {
        if self.map_type == MapType::Framed {
            let mut current_vpn = self.vpn_range.get_start();
            let len = data.len();
            let mut start = 0;
            loop {
                let src = &data[start..len.min(start + PAGE_SIZE)];
                let dst = &mut page_table
                    .vpn_to_pte(current_vpn)
                    .unwrap()
                    .ppn()
                    .get_bytes_array()[..src.len()];
                dst.copy_from_slice(src);
                start += PAGE_SIZE;
                if start >= len {
                    break;
                }
                current_vpn.step();
            }
        }
    }

    pub(crate) fn clone_data(&self, page_table: &PageTable, other: &MapArea) {
        if self.map_type == MapType::Framed {
            for vpn in other.vpn_range {
                let src = other
                    .data_frames
                    .get(&vpn)
                    .expect("internal error: vpn match failed")
                    .ppn
                    .get_bytes_array();
                let dst = page_table.vpn_to_pte(vpn).unwrap().ppn().get_bytes_array();
                dst.copy_from_slice(src);
            }
        }
    }
}

pub(crate) struct MemorySet {
    pub(crate) page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    pub(crate) fn empty() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    pub(crate) fn push_area(&mut self, mut map_area: MapArea, init_data: Option<&[u8]>) {
        map_area.reflect_self_to_page_table(&mut self.page_table);
        if let Some(data) = init_data {
            map_area.copy_data(&self.page_table, data);
        }
        self.areas.push(map_area);
    }

    fn fork_area(&mut self, other_area: &MapArea) {
        let range: super::address::SimpleRange<VirPageNum> = other_area.vpn_range;
        let mut map_area: MapArea = MapArea::new(
            VirAddr::from(range.get_start()),
            VirAddr::from(range.get_end()),
            other_area.map_type,
            other_area.map_per,
        );
        map_area.reflect_self_to_page_table(&mut self.page_table);
        map_area.clone_data(&self.page_table, other_area);
        self.areas.push(map_area);
    }

    // pub(crate) fn insert_framed_area(&mut self, start: VirAddr, end: VirAddr, permission: MapPermission) {
    //     self.areas.push(MapArea::new(start, end, MapType::Framed, permission));
    // }

    pub(crate) fn map_trampoline(&mut self) {
        extern "C" {
            fn __trampoline_start();
        }
        let va = VirAddr::from(TRAMPOLINE);
        let vpn: VirPageNum = va.into();
        self.page_table.build_reflect(
            VirAddr::from(TRAMPOLINE).into(),
            PhyAddr::from(__trampoline_start as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    pub(crate) fn build_kernel_space() -> Self {
        extern "C" {
            fn __text_start();
            fn __text_end();
            fn __rodata_start();
            fn __rodata_end();
            fn __data_start();
            fn __data_end();
            fn __bss_start_with_stack();
            fn __bss_end();
            fn __kernel_end();
            fn __trampoline_start();
        }
        let text_start = __text_start as usize;
        let text_end = __text_end as usize;
        let rodata_start = __rodata_start as usize;
        let rodata_end = __rodata_end as usize;
        let data_start = __data_start as usize;
        let data_end = __data_end as usize;
        let bss_start_with_stack = __bss_start_with_stack as usize;
        let bss_end = __bss_end as usize;
        let kernel_end = __kernel_end as usize;
        trace!(".text\t\t[{:#x}, {:#x})", text_start, text_end);
        trace!(".rodata\t[{:#x}, {:#x})", rodata_start, rodata_end);
        trace!(".data\t\t[{:#x}, {:#x})", data_start, data_end);
        trace!(
            ".bss\t\t[{:#x}, {:#x})",
            bss_start_with_stack as usize,
            bss_end as usize
        );
        trace!(".memory\t[{:#x}, {:#x})", __kernel_end as usize, MEMORY_END);
        let mut kernel_memory_set = Self::empty();
        kernel_memory_set.map_trampoline();
        kernel_memory_set.push_area(
            MapArea::new(
                VirAddr::from(text_start),
                VirAddr::from(text_end),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        kernel_memory_set.push_area(
            MapArea::new(
                VirAddr::from(rodata_start),
                VirAddr::from(rodata_end),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );
        kernel_memory_set.push_area(
            MapArea::new(
                VirAddr::from(data_start),
                VirAddr::from(data_end),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        kernel_memory_set.push_area(
            MapArea::new(
                VirAddr::from(bss_start_with_stack),
                VirAddr::from(bss_end),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        kernel_memory_set.push_area(
            MapArea::new(
                VirAddr::from(kernel_end),
                VirAddr::from(MEMORY_END),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        kernel_memory_set
    }

    pub(crate) fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = MemorySet::empty();
        memory_set.map_trampoline();
        let elf = xmas_elf::ElfFile::new(elf_data).expect("ELF parse error");
        let header = elf.header;
        let magic_num = header.pt1.magic;
        assert_eq!(magic_num, [0x7f, 0x45, 0x4c, 0x46], "invalid ELF");
        let mut max_end_vpn = VirPageNum(0);
        for i in 0..header.pt2.ph_count() {
            let area: xmas_elf::program::ProgramHeader<'_> = elf.program_header(i).unwrap();
            if area.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start = VirAddr::from(area.virtual_addr() as usize);
                let end = VirAddr::from(area.virtual_addr() as usize + area.mem_size() as usize);
                let mut permission: MapPermission = MapPermission::U;
                let flags = area.flags();
                if flags.is_read() {
                    permission |= MapPermission::R;
                }
                if flags.is_write() {
                    permission |= MapPermission::W;
                }
                if flags.is_execute() {
                    permission |= MapPermission::X;
                }
                let map_area = MapArea::new(start, end, MapType::Framed, permission);
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push_area(
                    map_area,
                    Some(
                        &elf.input
                            [area.offset() as usize..(area.offset() + area.file_size()) as usize],
                    ),
                );
            }
        }
        let max_end_va: VirAddr = max_end_vpn.into();
        let mut user_stack_btm_va: usize = max_end_va.into();
        // 在各段和栈底之间插入空白的守护页面
        user_stack_btm_va += PAGE_SIZE;
        let user_stack_top_va = user_stack_btm_va + USER_STACK_SIZE;
        memory_set.push_area(
            MapArea::new(
                user_stack_btm_va.into(),
                user_stack_top_va.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );
        memory_set.push_area(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        (
            memory_set,
            user_stack_top_va,
            elf.header.pt2.entry_point() as usize,
        )
    }

    // 没有设计为Clone trait，因为他涉及到数据的深拷贝
    pub(crate) fn fork_from(&mut self, other: &MemorySet) {
        for area in &other.areas {
            self.fork_area(area);
        }
    }

    pub(crate) fn activate(&self) {
        arch_relate::enable_virtual_address(self.page_table.address());
    }
}
