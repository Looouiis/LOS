use core::mem::MaybeUninit;

use alloc::{string::String, sync::Arc, vec::Vec};
use fs::{
    config::BLOCK_SIZE,
    structure::{Inode, OpenFlags},
    FileSystem,
};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{
    drivers::block::BLOCK_DEVICE,
    io::get_user_buf,
    mem::{address::VirAddr, page_table::ROTable},
    PROCESS_MANAGER,
};

lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = Arc::new({
        let fs = FileSystem::open(BLOCK_DEVICE.clone());
        Inode::get_root_inode(&fs)
    });
}

type UserBuffer = Vec<&'static mut [u8]>;

pub trait File {
    fn readable(&self) -> bool;
    fn writeable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
}

pub struct OSInode {
    readable: bool,
    writeable: bool,
    mutable: Mutex<OSInodeMutable>,
}

impl OSInode {
    pub fn new(readable: bool, writeable: bool, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writeable,
            mutable: Mutex::new(OSInodeMutable { offset: 0, inode }),
        }
    }

    #[allow(invalid_value)]
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.mutable.lock();
        let mut buffer: [u8; BLOCK_SIZE] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut res = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            res.extend_from_slice(&buffer);
            inner.offset += len;
        }
        res
    }
}

pub struct OSInodeMutable {
    offset: usize,
    inode: Arc<Inode>,
}

impl File for OSInode {
    fn readable(&self) -> bool {
        self.readable
    }

    fn writeable(&self) -> bool {
        self.writeable
    }

    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.mutable.lock();
        let mut total_len = 0;
        for slice in buf.iter_mut() {
            let len = inner.inode.read_at(inner.offset, *slice);
            if len == 0 {
                break;
            }
            inner.offset += len;
            total_len += len;
        }
        total_len
    }

    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.mutable.lock();
        let mut total_len = 0;
        for slice in buf.iter() {
            let len = inner.inode.write_at(inner.offset, *slice);
            if len == 0 {
                break;
            }
            inner.offset += len;
            total_len += len;
        }
        total_len
    }
}

pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writeable) = flags.read_write();
    match ROOT_INODE.find(name) {
        Some(inode) => {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear_data();
            }
            Some(Arc::new(OSInode::new(readable, writeable, inode)))
        }
        None => {
            if flags.contains(OpenFlags::CREATE) {
                ROOT_INODE
                    .create(name)
                    .map(|inode| Arc::new(OSInode::new(readable, writeable, inode)))
            } else {
                None
            }
        }
    }
}

pub(crate) fn sys_open(path: *const u8, flags: u32) -> isize {
    let openf_lags = OpenFlags::from_bits(flags).unwrap();
    let mut name = String::new();
    let mgr = PROCESS_MANAGER.get();
    let table = ROTable::from_token(mgr.get_current_token());
    let va = VirAddr::from(path as usize);
    loop {
        let pa = table.va_to_pa(va).unwrap();
        let ch = unsafe { *(pa.0 as *const char) };
        if ch == '\0' {
            break;
        }
        name.push(ch);
    }
    match open_file(name.as_str(), openf_lags) {
        Some(inode) => {
            let process = mgr.get_process().as_ref().unwrap();
            let mut guard = process.lock();
            let fd = guard.alloc_fd();
            guard.set_fd(fd, inode);
            fd as isize
        }
        None => -1,
    }
}

pub(crate) fn sys_close(fd: usize) -> isize {
    let mgr = PROCESS_MANAGER.get();
    let process = mgr.get_process().as_ref().unwrap();
    let mut guard = process.lock();
    if guard.dealloc_fd(fd) {
        0
    } else {
        -1
    }
}

pub(crate) fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let mgr = PROCESS_MANAGER.get();
    let process = mgr.get_process().as_ref().unwrap();
    let guard = process.lock();
    let table = ROTable::from_token(guard.get_token());
    match guard.get_fd(fd) {
        Some(file) => {
            let translated_buffer = get_user_buf(&table, buf, len);
            file.write(translated_buffer) as isize
        }
        None => -1,
    }
}

pub(crate) fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let mgr = PROCESS_MANAGER.get();
    let process = mgr.get_process().as_ref().unwrap();
    let guard = process.lock();
    let table = ROTable::from_token(guard.get_token());
    match guard.get_fd(fd) {
        Some(file) => {
            let translated_buffer = get_user_buf(&table, buf, len);
            file.read(translated_buffer) as isize
        }
        None => -1,
    }
}
