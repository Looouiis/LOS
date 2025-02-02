use core::mem::MaybeUninit;

use alloc::sync::Arc;

use crate::fs::config::BLOCK_SIZE;

use super::{cache::{BlockDevice, BLOCK_CACHE_MANAGER}, config::{EFS_MAGIC, INODE_DIRECT_COUNT, INODE_INDIRECT_COUNT}};

#[repr(C)]
pub(crate) struct SuperBlock {
    magic: u32,
    pub(crate) total_blocks_num: u32,
    pub(crate) inode_bitmap_offset: u32,
    pub(crate) inode_area_offset: u32,
    pub(crate) data_bitmap_offset: u32,
    pub(crate) data_area_offset: u32,
}

impl SuperBlock {
    pub(crate) fn init(
        &mut self,
        total_blocks_num: u32,
        inode_bitmap_offset: u32,
        inode_area_offset: u32,
        data_bitmap_offset: u32,
        data_area_offset: u32,
    ) {
        self.magic = EFS_MAGIC;
        self.total_blocks_num = total_blocks_num;
        self.inode_bitmap_offset = inode_bitmap_offset;
        self.inode_area_offset = inode_area_offset;
        self.data_bitmap_offset = data_bitmap_offset;
        self.data_area_offset = data_area_offset;
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

type BitMapBlock = [u64; 64];

#[repr(C)]
pub(crate) struct BitMap {
    start_block_id: usize,
    block_num: usize,
}

impl BitMap {
    pub(crate) fn init(&mut self, start: usize, num: usize) {
        self.start_block_id = start;
        self.block_num = num;
    }

    pub(crate) fn alloc(&mut self, device: Arc<dyn BlockDevice>) -> Option<usize> {
        let mut guard = BLOCK_CACHE_MANAGER.lock();
        for i in 0 .. self.block_num {
            if let Some(res) = guard.get_block(self.start_block_id + i, device.clone()).lock().modify::<BitMapBlock, Option<usize>>(0, |block| {
                for (index, u64unit) in block.iter_mut().enumerate() {
                    if *u64unit != u64::MAX {
                        let lowbit = *u64unit & (!*u64unit + 1);
                        *u64unit |= lowbit;
                        return Some(63 - lowbit.trailing_zeros() as usize + i * 512 + index * 64);
                    }
                }
                None
            }) {
                return Some(res);
            }
        }
        None
    }

    pub(crate) fn dealloc(&mut self, device: Arc<dyn BlockDevice>, pos: usize) {
        let block_idx = pos / 512;
        let block_offset = pos % 512;
        let unit_idx = block_offset / 64;
        let unit_offset = (block_offset % 64) as u64;
        BLOCK_CACHE_MANAGER.lock().get_block(self.start_block_id + block_idx, device).lock().modify(0, |block: &mut BitMapBlock| {
            assert!(block[unit_idx] & (1u64 << unit_offset) > 0);
            block[unit_idx] &= !(1u64 << unit_offset);
        })
    }
}

pub(crate) enum InodeType {
    File,
    Directory,
}

type IndirectBlock = [u32; BLOCK_SIZE / size_of::<u32>()];

// 128byte / Inode
#[repr(C)]
pub(crate) struct Inode {
    pub(crate) size: u32,
    pub(crate) direct_zone: [u32; INODE_DIRECT_COUNT],
    pub(crate) first_level_indirect: u32,
    pub(crate) second_level_indirect: u32,
    pub(crate) node_type: InodeType,
}

impl Inode {
    pub(crate) fn init(&mut self, node_type: InodeType) {
        self.size = 0;
        self.direct_zone = [0; INODE_DIRECT_COUNT];
        self.first_level_indirect = 0;      // 间接块中存的都是block地址（u32）
        self.second_level_indirect = 0;     // 间接块中存的都是block地址（u32）
        self.node_type = node_type;
    }

    pub(crate) fn get_block_id(&self, offset: u32, device: Arc<dyn BlockDevice>) -> u32 {
        let offset = offset as usize;
        if offset < INODE_DIRECT_COUNT {
            return self.direct_zone[offset];
        } else if offset < INODE_DIRECT_COUNT + INODE_INDIRECT_COUNT {
            let inner_offset = offset - INODE_DIRECT_COUNT;
            return BLOCK_CACHE_MANAGER.lock().get_block(self.first_level_indirect as usize, device).lock().read(inner_offset, |id: &u32| {
                return *id;
            });
        } else {
            let offset = offset - INODE_DIRECT_COUNT - INODE_INDIRECT_COUNT;
            let inner_idx = offset / INODE_INDIRECT_COUNT;
            let inner_offset = offset % INODE_INDIRECT_COUNT;
            let mut guard = BLOCK_CACHE_MANAGER.lock();
            return guard.get_block(self.second_level_indirect as usize, device.clone()).lock().read(inner_idx, |id: &u32| {
                return guard.get_block(*id as usize, device).lock().read(inner_offset, |id| {
                    return *id;
                })
            });
        }
    }
}
