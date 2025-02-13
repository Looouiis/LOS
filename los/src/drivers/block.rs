use alloc::sync::Arc;
use lazy_static::lazy_static;

use crate::{drivers::BlockDeviceImpl, fs::cache::BlockDevice};

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}
