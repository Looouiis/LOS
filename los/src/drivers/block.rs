use alloc::sync::Arc;
use fs::cache::BlockDevice;
use lazy_static::lazy_static;

use crate::drivers::BlockDeviceImpl;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}
