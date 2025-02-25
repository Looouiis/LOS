extern crate alloc;

use alloc::{string::String, sync::Arc, vec::Vec};
use spin::{Mutex, MutexGuard};

use crate::config::BLOCK_SIZE;

use super::{
    FileSystem,
    cache::{BLOCK_CACHE_MANAGER, BlockDevice},
    config::{EFS_MAGIC, INODE_DIRECT_COUNT, INODE_INDIRECT_COUNT},
};

#[repr(C)]
pub struct SuperBlock {
    magic: u32,
    pub total_block_num: u32,
    pub inode_bitmap_block_num: u32,
    pub inode_area_block_num: u32,
    pub data_bitmap_block_num: u32,
    pub data_area_block_num: u32,
}

impl SuperBlock {
    pub fn init(
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

    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

pub type BitMapBlock = [u64; 64];
pub type DataBlock = [u8; BLOCK_SIZE];

#[repr(C)]
pub struct BitMap {
    start_block_id: usize,
    block_num: usize,
}

impl BitMap {
    pub fn new(start_block_id: usize, block_num: usize) -> Self {
        Self {
            start_block_id,
            block_num,
        }
    }

    pub fn init(&mut self, start: usize, num: usize) {
        self.start_block_id = start;
        self.block_num = num;
    }

    pub fn alloc(&mut self, device: &Arc<dyn BlockDevice>) -> Option<usize> {
        let mut guard = BLOCK_CACHE_MANAGER.lock();
        for i in 0..self.block_num {
            if let Some(res) = guard
                .get_block(self.start_block_id + i, device)
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

    pub fn dealloc(&mut self, device: &Arc<dyn BlockDevice>, pos: usize) {
        let block_idx = pos / 512;
        let block_offset = pos % 512;
        let unit_idx = block_offset / 64;
        let unit_offset = block_offset % 64;
        BLOCK_CACHE_MANAGER
            .lock()
            .get_block(self.start_block_id + block_idx, device)
            .lock()
            .modify(0, |block: &mut BitMapBlock| {
                assert!(block[unit_idx] & (1u64 << unit_offset) > 0);
                block[unit_idx] &= !(1u64 << unit_offset);
            })
    }

    pub fn get_block_num(&self) -> usize {
        self.block_num
    }
}

#[derive(PartialEq)]
pub enum InodeType {
    File,
    Directory,
}

// type IndirectBlock = [u32; BLOCK_SIZE / size_of::<u32>()];

// 128byte / Inode
#[repr(C)]
pub struct DiskInode {
    pub size: u32,
    pub direct_zone: [u32; INODE_DIRECT_COUNT],
    pub first_level_indirect: u32,
    pub second_level_indirect: u32,
    pub node_type: InodeType,
}

impl DiskInode {
    pub fn init(&mut self, node_type: InodeType) {
        self.size = 0;
        self.direct_zone = [0; INODE_DIRECT_COUNT];
        self.first_level_indirect = 0; // 间接块中存的都是block地址（u32）
        self.second_level_indirect = 0; // 间接块中存的都是block地址（u32）
        self.node_type = node_type;
    }

    pub fn get_block_id(&self, offset: u32, device: &Arc<dyn BlockDevice>) -> u32 {
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
                .get_block(self.second_level_indirect as usize, device)
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

    pub fn data_blocks_num(size: u32) -> usize {
        (size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE
    }

    // data_block + indirect_index_block
    pub fn total_blocks_num(size: u32) -> usize {
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

    pub fn calc_new_blocks(&self, new_size: u32) -> usize {
        Self::total_blocks_num(new_size) - Self::total_blocks_num(self.size)
    }

    pub fn push_empty_blocks(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        device: &Arc<dyn BlockDevice>,
    ) {
        let cur = Self::total_blocks_num(self.size);
        let new = Self::total_blocks_num(new_size);
        let mut iter = new_blocks.into_iter().peekable();
        assert_eq!(new - cur, iter.len());
        let mut index = cur;
        // 未来优化：将for移动到各个分支中去（目前设想可以用iter）
        while iter.peek().is_some() {
            if index < INODE_DIRECT_COUNT {
                self.direct_zone[index] = iter.next().unwrap();
            } else if index < INODE_DIRECT_COUNT + INODE_INDIRECT_COUNT {
                let inner_idx = index - INODE_DIRECT_COUNT;
                if inner_idx == 0 {
                    self.first_level_indirect = iter.next().unwrap();
                }
                BLOCK_CACHE_MANAGER
                    .lock()
                    .get_block(self.first_level_indirect as usize, device)
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
                    .get_block(self.second_level_indirect as usize, device)
                    .lock()
                    .modify(inner_idx, |ptr: &mut u32| {
                        if inner_offset == 0 {
                            *ptr = iter.next().unwrap();
                        }
                        guard.get_block(*ptr as usize, device).lock().modify(
                            inner_offset,
                            |inner_ptr: &mut u32| {
                                *inner_ptr = iter.next().unwrap();
                            },
                        )
                    });
            }
            index += 1;
        }
        self.size = new_size;
    }

    pub fn clear_size(&mut self, device: &Arc<dyn BlockDevice>) -> Vec<u32> {
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
                    .get_block(self.first_level_indirect as usize, device)
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
                    .get_block(self.second_level_indirect as usize, device)
                    .lock()
                    .modify(inner_idx, |ptr: &mut u32| {
                        if inner_offset == 0 {
                            res.push(*ptr);
                        }
                        guard.get_block(*ptr as usize, device).lock().modify(
                            inner_offset,
                            |inner_ptr: &mut u32| {
                                res.push(*inner_ptr);
                            },
                        );
                    });
                if offset == 0 {
                    self.second_level_indirect = 0;
                }
            }
        }
        res
    }

    pub fn read_at(&self, start: usize, buf: &mut [u8], device: &Arc<dyn BlockDevice>) -> usize {
        let end = (start + buf.len()).min(self.size as usize);
        if end <= start {
            return 0;
        }
        let start_block = start / BLOCK_SIZE;
        let end_block = end / BLOCK_SIZE;

        // 展开首次，减少循环中分支判断
        let id = self.get_block_id(start_block as u32, device);
        let single_size = ((start_block + 1) * BLOCK_SIZE).min(end) - start;
        let mut guard = BLOCK_CACHE_MANAGER.lock();
        guard
            .get_block(id as usize, device)
            .lock()
            .read(0, |data: &[u8; BLOCK_SIZE]| {
                let src = &data[(start % BLOCK_SIZE)..(start % BLOCK_SIZE) + single_size];
                let dst = &mut buf[0..single_size];
                dst.copy_from_slice(src);
            });
        let mut read_size = single_size;

        drop(guard);
        for block_offset in (start_block + 1)..end_block {
            let id = self.get_block_id(block_offset as u32, device);
            let single_size =
                ((block_offset + 1) * BLOCK_SIZE).min(end) - block_offset * BLOCK_SIZE;
            let dst = &mut buf[read_size..(read_size + single_size)];
            BLOCK_CACHE_MANAGER
                .lock()
                .get_block(id as usize, device)
                .lock()
                .read(0, |data: &[u8; BLOCK_SIZE]| {
                    let src = &data[0..single_size];
                    dst.copy_from_slice(src);
                });
            read_size += single_size;
        }
        read_size
    }

    pub fn write_at(&self, start: usize, buf: &[u8], device: &Arc<dyn BlockDevice>) -> usize {
        let end = (start + buf.len()).min(self.size as usize);
        if end <= start {
            return 0;
        }
        let start_block = start / BLOCK_SIZE;
        let end_block = end / BLOCK_SIZE;

        // 展开首次，减少循环中分支判断
        let id = self.get_block_id(start_block as u32, device);
        let single_size = ((start_block + 1) * BLOCK_SIZE).min(end) - start;
        let mut guard = BLOCK_CACHE_MANAGER.lock();
        guard
            .get_block(id as usize, device)
            .lock()
            .modify(0, |data: &mut [u8; BLOCK_SIZE]| {
                let src = &buf[0..single_size];
                let dst = &mut data[(start % BLOCK_SIZE)..(start % BLOCK_SIZE) + single_size];
                dst.copy_from_slice(src);
            });
        let mut write_size = single_size;

        drop(guard);
        for block_offset in (start_block + 1)..end_block {
            let id = self.get_block_id(block_offset as u32, device);
            let single_size =
                ((block_offset + 1) * BLOCK_SIZE).min(end) - block_offset * BLOCK_SIZE;
            let src = &buf[write_size..(write_size + single_size)];
            BLOCK_CACHE_MANAGER
                .lock()
                .get_block(id as usize, device)
                .lock()
                .modify(0, |data: &mut [u8; BLOCK_SIZE]| {
                    let dst = &mut data[0..single_size];
                    dst.copy_from_slice(src);
                });
            write_size += single_size;
        }
        write_size
    }
}

const NAME_LENGTH_LIMIT: usize = 27;

pub struct DirEntry {
    name: [u8; NAME_LENGTH_LIMIT + 1],
    inode_id: u32,
}

impl DirEntry {
    pub fn empty() -> Self {
        Self {
            name: [0u8; NAME_LENGTH_LIMIT + 1],
            inode_id: 0,
        }
    }

    pub fn new(name: &str, inode_id: u32) -> Self {
        assert!(name.len() < NAME_LENGTH_LIMIT, "name {name} too long");
        let mut cloned: [u8; NAME_LENGTH_LIMIT + 1] = [0u8; NAME_LENGTH_LIMIT + 1];
        let dst = &mut cloned[0..name.len()];
        dst.copy_from_slice(name.as_bytes());
        Self {
            name: cloned,
            inode_id,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as usize as *const u8,
                size_of::<DirEntry>(),
            )
        }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *const _ as usize as *mut u8,
                size_of::<DirEntry>(),
            )
        }
    }

    pub fn get_name(&self) -> &str {
        for i in 0..size_of::<DirEntry>() {
            if self.name[i] == 0 {
                return core::str::from_utf8(&self.name[0..i]).unwrap();
            }
        }
        return core::str::from_utf8(&self.name).unwrap();
    }

    pub fn get_inode(&self) -> u32 {
        self.inode_id
    }
}

pub struct Inode {
    pub block_id: usize,
    pub block_offset: usize,
    pub fs: Arc<Mutex<FileSystem>>,
    pub device: Arc<dyn BlockDevice>,
}

impl Inode {
    pub fn new(
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

    pub fn get_root_inode(fs: &Arc<Mutex<FileSystem>>) -> Self {
        let guard = fs.lock();
        let (root_block_id, root_block_offset) = guard.get_disk_inode_pos_by_id(0);
        Self {
            block_id: root_block_id as usize,
            block_offset: root_block_offset,
            fs: fs.clone(),
            device: guard.device.clone(),
        }
    }

    pub fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        let mut mgr_guard = BLOCK_CACHE_MANAGER.lock();
        let block = mgr_guard.get_block(self.block_id, &self.device);
        drop(mgr_guard);
        block.lock().read(self.block_offset, f)
    }

    pub fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        let mut mgr_guard = BLOCK_CACHE_MANAGER.lock();
        let block = mgr_guard.get_block(self.block_id, &self.device);
        drop(mgr_guard);
        block.lock().modify(self.block_offset, f)
    }

    pub fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        assert!(disk_inode.node_type == InodeType::Directory);
        let file_cnt = disk_inode.size as usize / size_of::<DirEntry>();
        let mut dir = DirEntry::empty();
        for i in 0..file_cnt {
            let offset = i * size_of::<DirEntry>();
            if disk_inode.read_at(offset, dir.as_bytes_mut(), &self.device) == size_of::<DirEntry>()
            {
                if dir.get_name() == name {
                    return Some(dir.get_inode());
                }
            }
        }
        None
    }

    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
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

    pub fn ls(&self) -> Vec<String> {
        let mut res = Vec::new();
        let _guard = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            assert!(disk_inode.node_type == InodeType::Directory);
            let file_cnt = disk_inode.size as usize / size_of::<DirEntry>();
            let mut dir = DirEntry::empty();
            for i in 0..file_cnt {
                let offset = i * size_of::<DirEntry>();
                if disk_inode.read_at(offset, dir.as_bytes_mut(), &self.device)
                    == size_of::<DirEntry>()
                {
                    res.push(String::from(dir.get_name()));
                }
            }
        });
        res
    }

    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<FileSystem>,
    ) {
        let block_num = disk_inode.calc_new_blocks(new_size);
        let mut block_vec = Vec::new();
        for _ in 0..block_num {
            block_vec.push(fs.alloc_data_id());
        }
        disk_inode.push_empty_blocks(new_size, block_vec, &self.device);
    }

    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        // 判断文件是否已存在
        if self
            .read_disk_inode(|disk_inode| {
                assert!(disk_inode.node_type == InodeType::Directory);
                self.find_inode_id(name, &disk_inode)
            })
            .is_some()
        {
            return None;
        }
        // 申请目标文件的inode_id
        let new_id = fs.alloc_inode_id();
        // 修改本inode（类型为文件夹）中的目录项
        self.modify_disk_inode(|disk_inode| {
            let file_cnt = disk_inode.size as usize / size_of::<DirEntry>();
            let new_size = ((file_cnt + 1) * size_of::<DirEntry>()) as u32;
            self.increase_size(new_size, disk_inode, &mut fs);
            let dir = DirEntry::new(name, new_id);
            disk_inode.write_at(
                file_cnt * size_of::<DirEntry>(),
                dir.as_bytes(),
                &self.device,
            );
        });
        let (block_id, block_offset) = fs.get_disk_inode_pos_by_id(new_id);
        // 初始化目标文件的磁盘inode
        let mut guard = BLOCK_CACHE_MANAGER.lock();
        guard
            .get_block(block_id as usize, &self.device)
            .lock()
            .modify(block_offset, |new_inode: &mut DiskInode| {
                new_inode.init(InodeType::File);
            });
        guard.sync_all();
        // 构造目标文件的内存inode对象
        Some(Arc::new(Self::new(
            block_id as usize,
            block_offset,
            self.fs.clone(),
            self.device.clone(),
        )))
    }

    pub fn clear_data(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let need_dealloc = disk_inode.clear_size(&self.device);
            assert_eq!(
                need_dealloc.len(),
                DiskInode::total_blocks_num(disk_inode.size)
            );
            for block_id in need_dealloc {
                fs.dealloc_data_by_block_id(block_id);
            }
        });
    }

    pub fn read_at(&self, start: usize, buf: &mut [u8]) -> usize {
        let _guard = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(start, buf, &self.device))
    }

    pub fn write_at(&self, start: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            if start + buf.len() > disk_inode.size as usize {
                self.increase_size((start + buf.len()) as u32, disk_inode, &mut fs);
            }
            disk_inode.write_at(start, buf, &self.device)
        })
    }
}
