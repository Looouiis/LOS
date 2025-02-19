use alloc::vec::Vec;
use fs::cache::BlockDevice;
use spin::mutex::Mutex;
use virtio_drivers::{Hal, VirtIOBlk, VirtIOHeader};

use crate::{
    arch_relate,
    mem::{
        address::{PhyAddr, StepByOne, VirAddr},
        allocator::{frame::FrameTracker, FRAME_ALLOCATOR},
        memory_set::KERNEL_SPACE,
        page_table::ROTable,
    },
};

pub type BlockDeviceImpl = VirtIOBlock;

pub struct VirtIOBlock(Mutex<VirtIOBlk<'static, VirtioHal>>);

impl VirtIOBlock {
    pub(crate) fn new() -> Self {
        unsafe {
            Self(Mutex::new(
                VirtIOBlk::new(&mut *(MMIO[0].0 as *mut VirtIOHeader)).unwrap(),
            ))
        }
    }
}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .lock()
            .read_block(block_id, buf)
            .expect("VirtBlock read failed")
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .lock()
            .write_block(block_id, buf)
            .expect("VirtBlock write failed")
    }
}

static QUEUE_FRAMES: Mutex<Vec<FrameTracker>> = Mutex::new(Vec::new());

pub struct VirtioHal;

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> virtio_drivers::PhysAddr {
        let mut guard = QUEUE_FRAMES.lock();
        let frame = FRAME_ALLOCATOR.alloc().unwrap();
        let ppn_base = frame.ppn;
        guard.push(frame);
        for i in 1..pages {
            let frame = FRAME_ALLOCATOR.alloc_fresh().unwrap();
            assert_eq!(frame.ppn.0, ppn_base.0 + i);
            guard.push(frame);
        }
        let pa = PhyAddr::from(ppn_base);
        pa.0
    }

    fn dma_dealloc(paddr: virtio_drivers::PhysAddr, pages: usize) -> i32 {
        let mut ppn = PhyAddr::from(paddr).floor_to_ppn();
        for _i in 0..pages {
            FRAME_ALLOCATOR.dealloc(ppn);
            ppn.step();
        }
        todo!()
    }

    fn phys_to_virt(paddr: virtio_drivers::PhysAddr) -> virtio_drivers::VirtAddr {
        paddr
    }

    fn virt_to_phys(vaddr: virtio_drivers::VirtAddr) -> virtio_drivers::PhysAddr {
        let token = arch_relate::to_token(KERNEL_SPACE.get().page_table.address());
        let table = ROTable::from_token(token);
        table.va_to_pa(VirAddr::from(vaddr)).unwrap().0
    }
}

pub const MMIO: &[(usize, usize)] = &[(0x10001000, 0x1000)];
