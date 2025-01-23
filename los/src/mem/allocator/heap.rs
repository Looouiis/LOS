use core::{alloc::GlobalAlloc, ptr};

use spin::Mutex;

use crate::config::HEAP_SIZE;

#[link_section = ".data"]
pub(crate) static HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

pub struct BuddyAllocator {
    inner: Mutex<Heap>,
}

unsafe impl Sync for BuddyAllocator {}

unsafe impl GlobalAlloc for BuddyAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.inner.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.inner.lock().dealloc(ptr, layout);
    }
}

impl BuddyAllocator {
    pub const fn uninit() -> Self {
        Self {
            inner: Mutex::new(Heap::empty()),
        }
    }

    pub fn init(&self, start: usize, size: usize) {
        self.inner.lock().init(start, start + size);
    }

    #[allow(unused)]
    pub fn check_leak(&self) {
        self.inner.lock().check_leak();
    }
}

const LINK_LIST_NUM: usize = 32;

struct Heap {
    usize_list: [UsizeLinkedList; LINK_LIST_NUM],
    start: usize,
    alloc_cnt: usize,
}

impl Heap {
    const fn empty() -> Self {
        Self {
            usize_list: [UsizeLinkedList::new(); LINK_LIST_NUM],
            start: 0,
            alloc_cnt: 0,
        }
    }

    fn init(&mut self, start: usize, end: usize) {
        let aligned_start = (start + size_of::<usize>() - 1) & (!size_of::<usize>() + 1);
        let aligned_end = end & (!size_of::<usize>() + 1);
        self.start = aligned_start;
        let mut current_start = aligned_start;
        for i in (0..LINK_LIST_NUM).rev() {
            let judge = 1 << i;
            loop {
                let remain_size = aligned_end - current_start;
                if remain_size / judge > 0 {
                    unsafe { self.usize_list[i].push(current_start as *mut usize) }
                    current_start += judge;
                } else {
                    break;
                }
            }
        }
    }

    unsafe fn alloc(&mut self, layout: core::alloc::Layout) -> *mut u8 {
        let size = layout.size().next_power_of_two();
        let idx = size.trailing_zeros() as usize;
        for i in idx..=LINK_LIST_NUM {
            if !self.usize_list[i].is_empty() {
                for j in (idx + 1..=i).rev() {
                    match self.usize_list[j].pop() {
                        Some(head) => {
                            self.usize_list[j - 1]
                                .push((head as usize + (1 << (j - 1))) as *mut usize);
                            self.usize_list[j - 1].push(head);
                        }
                        None => panic!("internal error"),
                    }
                }
                break;
            }
            if i == LINK_LIST_NUM {
                return ptr::null_mut(); // 已经没有可用空间了
            }
        }
        assert!(!self.usize_list[idx].is_empty());
        match self.usize_list[idx].pop() {
            Some(res) => {
                self.alloc_cnt += 1;
                return res as *mut u8;
            }
            None => panic!("internal error"),
        }
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: core::alloc::Layout) {
        let size = layout.size().next_power_of_two();
        let idx = size.trailing_zeros() as usize;
        let mut final_ptr = ptr;
        for i in idx..LINK_LIST_NUM {
            let block_size = 1 << i;
            let buddy;
            let flag = ((final_ptr as usize - self.start) / block_size) % 2usize == 0;
            if flag {
                // buddy不合法（超出可用范围）也没根本上的问题，查找不到而已
                buddy = final_ptr as usize + block_size;
            } else {
                buddy = final_ptr as usize - block_size;
            }
            if self.usize_list[i].len() <= 1 && self.usize_list[i].find_and_pop_buddy(buddy) {
                if !flag {
                    final_ptr = buddy as *mut u8;
                }
            } else {
                self.usize_list[i].push(final_ptr as *mut usize);
                self.alloc_cnt -= 1;
                break;
            }
        }
    }

    fn dbg(&self) {
        for i in 0..LINK_LIST_NUM {
            if !self.usize_list[i].is_empty() {
                print!("\x1b[90m[kernel]: Block size: 0x{:<8x}\tdata: ", 1 << i);
                self.usize_list[i].dbg();
                println!("\x1b[0m");
            }
        }
    }

    pub(crate) fn check_leak(&self) {
        assert!(
            self.alloc_cnt == 0,
            "{} alloc hasn't been free yet",
            self.alloc_cnt
        );
    }
}

#[derive(Clone, Copy)]
struct UsizeLinkedList {
    head: *mut usize,
    len: usize,
}

impl UsizeLinkedList {
    const fn new() -> Self {
        Self {
            head: ptr::null_mut(),
            len: 0,
        }
    }

    unsafe fn push(&mut self, item: *mut usize) {
        *item = self.head as usize;
        self.head = item;
        self.len += 1;
    }

    unsafe fn pop(&mut self) -> Option<*mut usize> {
        if self.head.is_null() {
            None
        } else {
            let head = self.head;
            self.head = *self.head as *mut usize;
            self.len -= 1;
            Some(head)
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    fn dbg(&self) {
        let mut pointer: *mut usize = self.head;
        while !pointer.is_null() {
            unsafe {
                print!("0x{:<8x}   ->   ", pointer as usize);
                pointer = *pointer as *mut usize;
            }
        }
        print!("reach list end.");
    }

    unsafe fn find_and_pop_buddy(&mut self, buddy: usize) -> bool {
        if self.is_empty() {
            return false;
        }
        if self.head as usize == buddy {
            self.pop();
            return true;
        }
        let mut pointer: *mut usize = self.head;
        let mut nxt = *pointer as *mut usize;
        while !nxt.is_null() {
            unsafe {
                if nxt as usize == buddy {
                    let nxt_nxt = *nxt;
                    *pointer = nxt_nxt;
                    return true;
                }
                pointer = *pointer as *mut usize;
                nxt = *nxt as *mut usize;
            }
        }
        return false;
    }
}
