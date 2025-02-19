pub const BLOCK_SIZE: usize = 512;
pub const CACHE_NUM: usize = 16;
pub const EFS_MAGIC: u32 = 0x3b800001;
pub const INODE_DIRECT_COUNT: usize = 28;
pub const INODE_INDIRECT_COUNT: usize = BLOCK_SIZE / size_of::<u32>();
