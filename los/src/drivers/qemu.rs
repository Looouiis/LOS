use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;
use virtio_drivers::{Hal, VirtIOBlk};

use crate::fs::cache::BlockDevice;

type BlockDeviceImpl = VirtIOBlock;

pub struct VirtIOBlock(Mutex<VirtIOBlk<'static, VirtioHal>>);

pub struct VirtioHal;

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> virtio_drivers::PhysAddr {
        todo!()
    }

    fn dma_dealloc(paddr: virtio_drivers::PhysAddr, pages: usize) -> i32 {
        todo!()
    }

    fn phys_to_virt(paddr: virtio_drivers::PhysAddr) -> virtio_drivers::VirtAddr {
        todo!()
    }

    fn virt_to_phys(vaddr: virtio_drivers::VirtAddr) -> virtio_drivers::PhysAddr {
        todo!()
    }
}

pub const MMIO: &[(usize, usize)] = &[
    (0x10001000, 0x1000),
];

lazy_static!{
    // pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}