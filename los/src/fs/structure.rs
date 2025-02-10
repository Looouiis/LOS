use alloc::{string::String, sync::Arc, vec::Vec};
use spin::Mutex;

use crate::fs::config::BLOCK_SIZE;

use super::{
    cache::{BlockDevice, BLOCK_CACHE_MANAGER},
    config::{EFS_MAGIC, INODE_DIRECT_COUNT, INODE_INDIRECT_COUNT},
    FileSystem,
};

#[repr(C)]
pub(crate) struct SuperBlock {
    magic: u32,
    pub(crate) total_block_num: u32,
    pub(crate) inode_bitmap_block_num: u32,
    pub(crate) inode_area_block_num: u32,
    pub(crate) data_bitmap_block_num: u32,
    pub(crate) data_area_block_num: u32,
}

impl SuperBlock {
    pub(crate) fn init(
        &mut self,
        total_block_num: u32,
        inode_bitmap_block_num: u32,
        inode_area_block_num: u32,
        data_bitmap_block_num: u32,
        data_area_block_num: u32,
    ) {
        self.magic = EFS_MAGIC;
        self.total_block_num = total_block_num;
        self.inode_bitmap_block_num = inode_bitmap_block_num;
        self.inode_area_block_num = inode_area_block_num;
        self.data_bitmap_block_num = data_bitmap_block_num;
        self.data_area_block_num = data_area_block_num;
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

pub(crate) type BitMapBlock = [u64; 64];
pub(crate) type DataBlock = [u8; BLOCK_SIZE];

#[repr(C)]
pub(crate) struct BitMap {
    start_block_id: usize,
    block_num: usize,
}

impl BitMap {
    pub(crate) fn new(start_block_id: usize, block_num: usize) -> Self {
        Self {
            start_block_id,
            block_num,
        }
    }

    pub(crate) fn init(&mut self, start: usize, num: usize) {
        self.start_block_id = start;
        self.block_num = num;
    }

    pub(crate) fn alloc(&mut self, device: &Arc<dyn BlockDevice>) -> Option<usize> {
        let mut guard = BLOCK_CACHE_MANAGER.lock();
        for i in 0..self.block_num {
            if let Some(res) = guard
                .get_block(self.start_block_id + i, device.clone())
                .lock()
                .modify::<BitMapBlock, Option<usize>>(0, |block| {
                    for (index, u64unit) in block.iter_mut().enumerate() {
                        if *u64unit != u64::MAX {
                            let unit_offset = u64unit.trailing_ones();
                            *u64unit |= 1u64 << unit_offset;
                            return Some(i * 512 + index * 64 + unit_offset as usize);
                        }
                    }
                    None
                })
            {
                return Some(res);
            }
        }
        None
    }

    pub(crate) fn dealloc(&mut self, device: &Arc<dyn BlockDevice>, pos: usize) {
        let block_idx = pos / 512;
        let block_offset = pos % 512;
        let unit_idx = block_offset / 64;
        let unit_offset = block_offset % 64;
        BLOCK_CACHE_MANAGER
            .lock()
            .get_block(self.start_block_id + block_idx, device.clone())
            .lock()
            .modify(0, |block: &mut BitMapBlock| {
                assert!(block[unit_idx] & (1u64 << unit_offset) > 0);
                block[unit_idx] &= !(1u64 << unit_offset);
            })
    }
}

#[derive(PartialEq)]
pub(crate) enum InodeType {
    File,
    Directory,
}

type IndirectBlock = [u32; BLOCK_SIZE / size_of::<u32>()];

// 128byte / Inode
#[repr(C)]
pub(crate) struct DiskInode {
    pub(crate) size: u32,
    pub(crate) direct_zone: [u32; INODE_DIRECT_COUNT],
    pub(crate) first_level_indirect: u32,
    pub(crate) second_level_indirect: u32,
    pub(crate) node_type: InodeType,
}

impl DiskInode {
    pub(crate) fn init(&mut self, node_type: InodeType) {
        self.size = 0;
        self.direct_zone = [0; INODE_DIRECT_COUNT];
        self.first_level_indirect = 0; // 间接块中存的都是block地址（u32）
        self.second_level_indirect = 0; // 间接块中存的都是block地址（u32）
        self.node_type = node_type;
    }

    pub(crate) fn get_block_id(&self, offset: u32, device: Arc<dyn BlockDevice>) -> u32 {
        let offset = offset as usize;
        if offset < INODE_DIRECT_COUNT {
            return self.direct_zone[offset];
        } else if offset < INODE_DIRECT_COUNT + INODE_INDIRECT_COUNT {
            let inner_offset = offset - INODE_DIRECT_COUNT;
            return BLOCK_CACHE_MANAGER
                .lock()
                .get_block(self.first_level_indirect as usize, device)
                .lock()
                .read(inner_offset, |id: &u32| {
                    return *id;
                });
        } else {
            let offset = offset - INODE_DIRECT_COUNT - INODE_INDIRECT_COUNT;
            let inner_idx = offset / INODE_INDIRECT_COUNT;
            let inner_offset = offset % INODE_INDIRECT_COUNT;
            let mut guard = BLOCK_CACHE_MANAGER.lock();
            return guard
                .get_block(self.second_level_indirect as usize, device.clone())
                .lock()
                .read(inner_idx, |id: &u32| {
                    return guard
                        .get_block(*id as usize, device)
                        .lock()
                        .read(inner_offset, |id| {
                            return *id;
                        });
                });
        }
    }

    pub(crate) fn data_blocks_num(size: u32) -> usize {
        (size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE
    }

    // data_block + indirect_index_block
    pub(crate) fn total_blocks_num(size: u32) -> usize {
        let data_blocks_num = Self::data_blocks_num(size);
        let mut res = data_blocks_num;
        if data_blocks_num > INODE_DIRECT_COUNT {
            res += 1;
        }
        if data_blocks_num > INODE_DIRECT_COUNT + INODE_INDIRECT_COUNT {
            res += 1;
            let remains = data_blocks_num - INODE_DIRECT_COUNT - INODE_INDIRECT_COUNT;
            res += (remains + INODE_INDIRECT_COUNT - 1) / INODE_INDIRECT_COUNT
        }
        res
    }

    pub(crate) fn calc_new_blocks(&self, new_size: u32) -> usize {
        Self::total_blocks_num(new_size) - Self::total_blocks_num(self.size)
    }

    pub(crate) fn push_empty_blocks(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        device: Arc<dyn BlockDevice>,
    ) {
        let cur = Self::data_blocks_num(self.size);
        let new = Self::data_blocks_num(new_size);
        let mut iter = new_blocks.into_iter();
        assert!(new - cur == iter.len());
        // 未来优化：将for移动到各个分支中去（目前设想可以用iter）
        for i in 0..(new - cur) {
            let index = cur + i;
            if index < INODE_DIRECT_COUNT {
                self.direct_zone[index] = iter.next().unwrap();
            } else if index < INODE_DIRECT_COUNT + INODE_INDIRECT_COUNT {
                let inner_idx = index - INODE_DIRECT_COUNT;
                if inner_idx == 0 {
                    self.first_level_indirect = iter.next().unwrap();
                }
                BLOCK_CACHE_MANAGER
                    .lock()
                    .get_block(self.first_level_indirect as usize, device.clone())
                    .lock()
                    .modify(inner_idx, |ptr: &mut u32| {
                        *ptr = iter.next().unwrap();
                    });
            } else {
                let offset = index - INODE_DIRECT_COUNT - INODE_INDIRECT_COUNT;
                let inner_idx = offset / INODE_INDIRECT_COUNT;
                let inner_offset: usize = offset % INODE_INDIRECT_COUNT;
                if offset == 0 {
                    self.second_level_indirect = iter.next().unwrap();
                }
                let mut guard = BLOCK_CACHE_MANAGER.lock();
                guard
                    .get_block(self.second_level_indirect as usize, device.clone())
                    .lock()
                    .modify(inner_idx, |ptr: &mut u32| {
                        if inner_offset == 0 {
                            *ptr = iter.next().unwrap();
                        }
                        guard
                            .get_block(*ptr as usize, device.clone())
                            .lock()
                            .modify(inner_offset, |inner_ptr: &mut u32| {
                                *inner_ptr = iter.next().unwrap();
                            })
                    });
            }
        }
    }

    pub(crate) fn clear_size(&mut self, device: Arc<dyn BlockDevice>) -> Vec<u32> {
        let mut res = Vec::new();
        let cur = Self::data_blocks_num(self.size);
        // 未来优化：将for移动到各个分支中去（目前设想可以用iter）
        for index in 0..cur {
            if index < INODE_DIRECT_COUNT {
                res.push(self.direct_zone[index]);
            } else if index < INODE_DIRECT_COUNT + INODE_INDIRECT_COUNT {
                let inner_idx = index - INODE_DIRECT_COUNT;
                if inner_idx == 0 {
                    res.push(self.first_level_indirect);
                }
                BLOCK_CACHE_MANAGER
                    .lock()
                    .get_block(self.first_level_indirect as usize, device.clone())
                    .lock()
                    .modify(inner_idx, |ptr: &mut u32| {
                        res.push(*ptr);
                    });
                if inner_idx == 0 {
                    self.first_level_indirect = 0;
                }
            } else {
                let offset = index - INODE_DIRECT_COUNT - INODE_INDIRECT_COUNT;
                let inner_idx = offset / INODE_INDIRECT_COUNT;
                let inner_offset: usize = offset % INODE_INDIRECT_COUNT;
                if offset == 0 {
                    res.push(self.second_level_indirect);
                }
                let mut guard = BLOCK_CACHE_MANAGER.lock();
                guard
                    .get_block(self.second_level_indirect as usize, device.clone())
                    .lock()
                    .modify(inner_idx, |ptr: &mut u32| {
                        if inner_offset == 0 {
                            res.push(*ptr);
                        }
                        guard
                            .get_block(*ptr as usize, device.clone())
                            .lock()
                            .modify(inner_offset, |inner_ptr: &mut u32| {
                                res.push(*inner_ptr);
                            });
                    });
                if offset == 0 {
                    self.second_level_indirect = 0;
                }
            }
        }
        res
    }

    pub(crate) fn read_at(
        &self,
        start: usize,
        buf: &mut [u8],
        device: Arc<dyn BlockDevice>,
    ) -> usize {
        let end = (start + buf.len()).min(self.size as usize);
        if end <= start {
            return 0;
        }
        let start_block = start / BLOCK_SIZE;
        let end_block = end / BLOCK_SIZE;
        let mut guard = BLOCK_CACHE_MANAGER.lock();

        // 展开首次，减少循环中分支判断
        let id = self.get_block_id(start_block as u32, device.clone());
        let single_size = ((start_block + 1) * BLOCK_SIZE).min(end) - start;
        guard
            .get_block(id as usize, device.clone())
            .lock()
            .read(0, |data: &[u8; BLOCK_SIZE]| {
                let src = &data[start..start + single_size];
                let dst = &mut buf[0..single_size];
                dst.copy_from_slice(src);
            });
        let mut read_size = single_size;

        for block_offset in (start_block + 1)..end_block {
            let id = self.get_block_id(block_offset as u32, device.clone());
            let single_size =
                ((block_offset + 1) * BLOCK_SIZE).min(end) - block_offset * BLOCK_SIZE;
            let dst = &mut buf[read_size..(read_size + single_size)];
            guard.get_block(id as usize, device.clone()).lock().read(
                0,
                |data: &[u8; BLOCK_SIZE]| {
                    let src = &data[0..single_size];
                    dst.copy_from_slice(src);
                },
            );
            read_size += single_size;
        }
        read_size
    }

    pub(crate) fn write_at(
        &self,
        start: usize,
        buf: &mut [u8],
        device: Arc<dyn BlockDevice>,
    ) -> usize {
        let end = (start + buf.len()).min(self.size as usize);
        if end <= start {
            return 0;
        }
        let start_block = start / BLOCK_SIZE;
        let end_block = end / BLOCK_SIZE;
        let mut guard = BLOCK_CACHE_MANAGER.lock();

        // 展开首次，减少循环中分支判断
        let id = self.get_block_id(start_block as u32, device.clone());
        let single_size = ((start_block + 1) * BLOCK_SIZE).min(end) - start;
        guard.get_block(id as usize, device.clone()).lock().modify(
            0,
            |data: &mut [u8; BLOCK_SIZE]| {
                let src = &buf[0..single_size];
                let dst = &mut data[start..start + single_size];
                dst.copy_from_slice(src);
            },
        );
        let mut write_size = single_size;

        for block_offset in (start_block + 1)..end_block {
            let id = self.get_block_id(block_offset as u32, device.clone());
            let single_size =
                ((block_offset + 1) * BLOCK_SIZE).min(end) - block_offset * BLOCK_SIZE;
            let src = &buf[write_size..(write_size + single_size)];
            guard.get_block(id as usize, device.clone()).lock().modify(
                0,
                |data: &mut [u8; BLOCK_SIZE]| {
                    let dst = &mut data[0..single_size];
                    dst.copy_from_slice(src);
                },
            );
            write_size += single_size;
        }
        write_size
    }
}

const NAME_LENGTH_LIMIT: usize = 27;

pub(crate) struct DirEntry {
    name: [u8; NAME_LENGTH_LIMIT + 1],
    inode_id: u32,
}

impl DirEntry {
    pub(crate) fn empty() -> Self {
        Self {
            name: [0u8; NAME_LENGTH_LIMIT + 1],
            inode_id: 0,
        }
    }

    pub(crate) fn new(name: &str, inode_id: u32) -> Self {
        let mut cloned: [u8; NAME_LENGTH_LIMIT + 1] = [0u8; NAME_LENGTH_LIMIT + 1];
        let dst = &mut cloned[0..name.len()];
        dst.copy_from_slice(name.as_bytes());
        Self {
            name: cloned,
            inode_id,
        }
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as usize as *const u8,
                size_of::<DirEntry>(),
            )
        }
    }
    pub(crate) fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *const _ as usize as *mut u8,
                size_of::<DirEntry>(),
            )
        }
    }

    pub(crate) fn get_name(&self) -> &str {
        for i in 0..size_of::<DirEntry>() {
            if self.name[i] == 0 {
                return core::str::from_utf8(&self.name[0..i]).unwrap();
            }
        }
        return core::str::from_utf8(&self.name).unwrap();
    }

    pub(crate) fn get_inode(&self) -> u32 {
        self.inode_id
    }
}

pub(crate) struct Inode {
    pub(crate) block_id: usize,
    pub(crate) block_offset: usize,
    pub(crate) fs: Arc<Mutex<FileSystem>>,
    pub(crate) device: Arc<dyn BlockDevice>,
}

impl Inode {
    pub(crate) fn new(
        block_id: usize,
        block_offset: usize,
        fs: Arc<Mutex<FileSystem>>,
        device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id,
            block_offset,
            fs,
            device,
        }
    }

    pub(crate) fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        BLOCK_CACHE_MANAGER
            .lock()
            .get_block(self.block_id, self.device.clone())
            .lock()
            .read(self.block_offset, f)
    }

    pub(crate) fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        BLOCK_CACHE_MANAGER
            .lock()
            .get_block(self.block_id, self.device.clone())
            .lock()
            .modify(self.block_offset, f)
    }

    pub(crate) fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        assert!(disk_inode.node_type == InodeType::Directory);
        let file_cnt = disk_inode.size as usize / size_of::<DirEntry>();
        let mut dir = DirEntry::empty();
        for i in 0..file_cnt {
            let offset = i * size_of::<DirEntry>();
            if disk_inode.read_at(offset, dir.as_bytes_mut(), self.device.clone())
                == size_of::<DirEntry>()
            {
                if dir.get_name() == name {
                    return Some(dir.get_inode());
                }
            }
        }
        None
    }

    pub(crate) fn find(&self, name: &str) -> Option<Arc<Inode>> {
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = self.fs.lock().get_disk_inode_pos_by_id(inode_id);
                Arc::new(Inode::new(
                    block_id as usize,
                    block_offset,
                    self.fs.clone(),
                    self.device.clone(),
                ))
            })
        })
    }

    pub(crate) fn ls(&self) -> Vec<String> {
        let mut res = Vec::new();
        let _guard = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            assert!(disk_inode.node_type == InodeType::Directory);
            let file_cnt = disk_inode.size as usize / size_of::<DirEntry>();
            let mut dir = DirEntry::empty();
            for i in 0..file_cnt {
                let offset = i * size_of::<DirEntry>();
                if disk_inode.read_at(offset, dir.as_bytes_mut(), self.device.clone())
                    == size_of::<DirEntry>()
                {
                    res.push(String::from(dir.get_name()));
                }
            }
        });
        res
    }
}
