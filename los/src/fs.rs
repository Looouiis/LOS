use alloc::sync::Arc;
use cache::{BlockDevice, BLOCK_CACHE_MANAGER};
use config::BLOCK_SIZE;
use spin::mutex::Mutex;
use structure::{BitMap, DataBlock, DiskInode, SuperBlock};

pub mod cache;
pub mod config;
pub mod structure;

pub(crate) struct FileSystem {
    pub(crate) device: Arc<dyn BlockDevice>,
    pub(crate) inode_bitmap: BitMap,
    pub(crate) data_bitmap: BitMap,
    inode_area_start_block: u32,
    data_area_start_block: u32,
}

impl FileSystem {
    pub(crate) fn create(
        device: Arc<dyn BlockDevice>,
        total_block_num: u32,
        inode_bitmap_block_num: u32,
    ) -> Arc<Mutex<Self>> {
        let inode_num = inode_bitmap_block_num as usize * BLOCK_SIZE * 8;
        let inode_area_block_num =
            (inode_num * size_of::<DiskInode>() + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let inode_total_block_num = inode_bitmap_block_num as usize + inode_area_block_num;

        let data_total_block_num = total_block_num as usize - inode_total_block_num - 1;
        // 每个数据块占用的位数
        const SINGLE_BLOCK_OCCUPY: usize = BLOCK_SIZE * 8 + 1;
        let data_bitmap_block_num =
            (data_total_block_num + SINGLE_BLOCK_OCCUPY - 1) / SINGLE_BLOCK_OCCUPY;
        let data_area_block_num = data_total_block_num - data_bitmap_block_num;

        let mut guard = BLOCK_CACHE_MANAGER.lock();
        for index in 0..total_block_num as usize {
            guard
                .get_block(index, device.clone())
                .lock()
                .modify(0, |block: &mut DataBlock| {
                    for ptr in block {
                        *ptr = 0;
                    }
                })
        }
        guard
            .get_block(0, device.clone())
            .lock()
            .modify(0, |block: &mut SuperBlock| {
                block.init(
                    total_block_num,
                    inode_bitmap_block_num,
                    inode_area_block_num as u32,
                    data_bitmap_block_num as u32,
                    data_area_block_num as u32,
                );
            });

        let fs = Self {
            device,
            inode_bitmap: BitMap::new(1, inode_bitmap_block_num as usize),
            data_bitmap: BitMap::new(1 + inode_total_block_num, data_bitmap_block_num),
            inode_area_start_block: 1 + inode_bitmap_block_num,
            data_area_start_block: (1 + inode_total_block_num + data_bitmap_block_num) as u32,
        };
        unimplemented!();
        Arc::new(Mutex::new(fs))
    }
}
