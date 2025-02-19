extern crate alloc;

use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use core::{any::Any, mem::MaybeUninit};
use spin::mutex::Mutex;

use super::config::{BLOCK_SIZE, CACHE_NUM};

pub static BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> = Mutex::new(BlockCacheManager::new());

pub trait BlockDevice: Send + Sync + Any {
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    fn write_block(&self, block_id: usize, buf: &[u8]);
}

pub struct BlockCache {
    cache: [u8; BLOCK_SIZE],
    block_id: usize,
    device: Arc<dyn BlockDevice>,
    dirt: bool,
}

impl BlockCache {
    #[allow(invalid_value)]
    pub fn new(block_id: usize, device: Arc<dyn BlockDevice>) -> Self {
        let mut cache: [u8; BLOCK_SIZE] = unsafe { MaybeUninit::uninit().assume_init() };
        device.read_block(block_id, &mut cache);
        Self {
            cache,
            block_id,
            device,
            dirt: false,
        }
    }

    pub fn get_ptr(&self, offset: usize) -> usize {
        self.cache.as_ptr() as usize + offset
    }

    pub fn get_ref<T: Sized>(&self, offset: usize) -> &T {
        assert!(size_of::<T>() + offset <= BLOCK_SIZE);
        let ptr = self.get_ptr(offset) as *const T;
        unsafe { &(*ptr) }
    }

    pub fn get_mut<T: Sized>(&mut self, offset: usize) -> &mut T {
        assert!(size_of::<T>() + offset <= BLOCK_SIZE);
        let ptr = self.get_ptr(offset) as *mut T;
        self.dirt = true;
        unsafe { &mut (*ptr) }
    }

    pub fn sync(&mut self) {
        if self.dirt {
            self.device.write_block(self.block_id, &self.cache);
        }
    }

    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync();
    }
}

pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
    pub const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn get_block(
        &mut self,
        id: usize,
        device: &Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        match self.queue.iter().find(|pair| pair.0 == id) {
            Some(pair) => pair.1.clone(),
            None => {
                if self.queue.len() == CACHE_NUM {
                    if let Some((index, _pair)) = self
                        .queue
                        .iter()
                        .enumerate()
                        .find(|(_, pair)| Arc::strong_count(&pair.1) == 1)
                    {
                        self.queue.remove(index);
                    } else {
                        panic!("暂时先不负责");
                    }
                }
                let cache = Arc::new(Mutex::new(BlockCache::new(id, device.clone())));
                self.queue.push_back((id, cache.clone()));
                cache
            }
        }
    }

    pub fn sync_all(&self) {
        for (_, cache) in self.queue.iter() {
            cache.lock().sync();
        }
    }
}
