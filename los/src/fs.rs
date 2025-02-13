use alloc::{sync::Arc, vec::Vec};
use cache::{BlockDevice, BLOCK_CACHE_MANAGER};
use config::BLOCK_SIZE;
use spin::mutex::Mutex;
use structure::{BitMap, DataBlock, DiskInode, Inode, InodeType, SuperBlock};

pub mod cache;
pub mod config;
pub mod structure;

pub trait File {
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
}

pub struct UserBuffer {
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    pub(crate) fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }

    pub(crate) fn len(&self) -> usize {
        self.buffers.iter().map(|item| item.len()).sum()
    }
}

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
                .get_block(index, &device)
                .lock()
                .modify(0, |block: &mut DataBlock| {
                    for ptr in block {
                        *ptr = 0;
                    }
                })
        }
        guard
            .get_block(0, &device)
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

        let mut fs = Self {
            device: device.clone(),
            inode_bitmap: BitMap::new(1, inode_bitmap_block_num as usize),
            data_bitmap: BitMap::new(1 + inode_total_block_num, data_bitmap_block_num),
            inode_area_start_block: 1 + inode_bitmap_block_num,
            data_area_start_block: (1 + inode_total_block_num + data_bitmap_block_num) as u32,
        };
        assert!(fs.inode_bitmap.alloc(&device) == Some(0));
        let (block_id, block_offset) = fs.get_disk_inode_pos_by_id(0);
        guard.get_block(block_id as usize, &device).lock().modify(
            block_offset,
            |root_inode: &mut DiskInode| {
                root_inode.init(InodeType::Directory);
            },
        );
        guard.sync_all();
        Arc::new(Mutex::new(fs))
    }

    fn get_disk_inode_pos_by_id(&self, inode_id: u32) -> (u32, usize) {
        let bit_length = inode_id * size_of::<DiskInode>() as u32;
        let block = self.inode_area_start_block + bit_length / BLOCK_SIZE as u32;
        let offset = bit_length as usize % BLOCK_SIZE;
        (block, offset)
    }

    fn get_block_id_by_data_id(&self, data_block_id: u32) -> u32 {
        self.data_area_start_block + data_block_id
    }

    fn alloc_inode_id(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.device).unwrap() as u32
    }

    fn alloc_data_id(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.device).unwrap() as u32
    }

    fn dealloc_data_id(&mut self, data_id: u32) {
        self.dealloc_data_by_block_id(self.get_block_id_by_data_id(data_id));
    }

    fn dealloc_data_by_block_id(&mut self, block_id: u32) {
        BLOCK_CACHE_MANAGER
            .lock()
            .get_block(block_id as usize, &self.device)
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                for ptr in data_block {
                    *ptr = 0;
                }
            });
        self.data_bitmap.dealloc(
            &self.device,
            (block_id - self.data_area_start_block) as usize,
        );
    }

    pub(crate) fn get_root_inode(fs: &Arc<Mutex<Self>>) -> Inode {
        let guard = fs.lock();
        let device = guard.device.clone();
        let (block_id, block_offset) = guard.get_disk_inode_pos_by_id(0);
        Inode {
            block_id: block_id as usize,
            block_offset,
            fs: fs.clone(),
            device,
        }
    }
}
