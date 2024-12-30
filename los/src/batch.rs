use core::{cell::{RefCell, RefMut}, cmp::min, sync::atomic::AtomicBool};
use lazy_static::lazy_static;

use crate::{arch_relate::{self, syscall_handler::trap::{trap_restore, TrapContext}}, stack::{UserStack, USER_STACK, USER_STACK_SIZE}};

extern "C" {
    fn _num_app();
}

pub(crate) static RUNNING: AtomicBool = AtomicBool::new(false);

pub(crate) const MAX_APP_NUM: usize = 20;

lazy_static!{
    pub(crate) static ref APP_MANAGER: ArcCell<AppManager> = unsafe {
        ArcCell::new({
            let ptr = _num_app as usize as *const usize;
            let app_num = ptr.read_volatile();
            let process: [Process; MAX_APP_NUM + 1] = [Process{pc: 0, start: 0, len: 0, status: State::Exited, stack: None, ctx: TrapContext::new()}; MAX_APP_NUM + 1];
            // let start_slice = core::slice::from_raw_parts(ptr.add(1), app_num + 1);
            // process[..= app_num].copy_from_slice(start_slice);
            // for i in 0 ..= app_num {
            //     let start = ptr.add(1 + i).read_volatile();
            //     process[i].start = start;
            //     process[i].pc = start;
            //     process[i].len = AppManager::LIMIT;.
            // }
            AppManager {
                app_num,
                current_app: 0,
                process,
                kernel_ctx: TrapContext::new(),
                current_entry: AppManager::ENTRY,
            }
        })
    };
}

#[derive(Clone, Copy)]
pub(crate) struct Process {
    pub(crate) start: usize,
    pub(crate) pc: usize,
    pub(crate) len: usize,
    pub(crate) status: State,
    // stack: Option<UserStack>,
    pub(crate) stack: Option<usize>,
    pub(crate) ctx: TrapContext,
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Ready, Run, Finish,
    Exited
}

pub(crate) struct AppManager {
    app_num: usize,
    current_app: usize,
    // app_start: [usize; MAX_APP_NUM + 1],
    process: [Process; MAX_APP_NUM + 1],
    pub(crate) kernel_ctx: TrapContext,
    current_entry: usize,
}

impl AppManager {
    pub(crate) const ENTRY: usize = 0x80400000;
    pub(crate) const LIMIT: usize = 0x20000;

    pub fn print_info(&self) {
        log!("app_num: {}", self.app_num);
    }

    unsafe fn load_app(&mut self) {
        let ptr = _num_app as usize as *const usize;
        let app_num = ptr.read_volatile();
        for i in 0 ..= app_num {
            let start = ptr.add(1 + i).read_volatile();
            self.process[i].start = start;
            self.process[i].pc = start;
            self.process[i].len = AppManager::LIMIT;
            self.process[i].status = State::Ready;
            self.process[i].stack = Some(i);
            self.process[i].ctx.info.sp = USER_STACK[i].get_sp_top();
        }
        (0 .. self.app_num).for_each(|id| {
            let mut ptr = (Self::ENTRY + id * Self::LIMIT) as *mut u8;
            (self.process[id].start .. self.process[id].start + self.process[id].len).for_each(|raw| {
                let ch = (raw as *mut u8).read_volatile();
                ptr.write_volatile(ch);
                ptr = ptr.add(1);
            });
            fence!();
        });
    }

    // 将app_manager内部的指针转移指向下一个app
    fn nxt_app(&mut self) -> bool {
        let mut index = 0;
        while index < self.app_num {
            if self.process[index].status == State::Ready {
                break
            }
            index += 1;
        }
        if index == self.app_num {
            log!("Execute complete");
            false
        }
        else {
            self.current_app = index;
            self.current_entry = Self::ENTRY + Self::LIMIT * index;
            true
        }
        // (0 .. self.app_num).for_each(|i| {
        //     if self.process[i].status == State::Ready {
        //         self.current_app = i;
        //         self.current_entry = Self::ENTRY + Self::LIMIT * i;
        //         return true;
        //     }
        // });
        // log!("Execute complete");
        // return false;
    }

    // pub(crate) fn get_entry(&self) -> usize {
    //     self.current_entry
    // }

    pub(crate) fn get_process(&self) -> &Process {
        &self.process[self.current_app]
    }

    pub(crate) fn exit_current(&mut self) {
        self.process[self.current_app].status = State::Exited;
    }

    pub(crate) fn mark_pc(&mut self, pc: usize) {
        self.process[self.current_app].pc = pc;
    }

    pub(crate) fn mark_ctx(&mut self, ctx: TrapContext) {
        self.process[self.current_app].ctx = ctx;
    }
}

unsafe impl<T> Sync for ArcCell<T> {}

pub(crate) struct ArcCell<T> {
    inner: RefCell<T>
}

impl<T> ArcCell<T> {
    fn new(item: T) -> Self {
        Self { inner: RefCell::new(item) }
    }

    pub fn get(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}

#[no_mangle]
pub(crate) fn exit(code: usize) -> ! {
    log!("function exit with {}", code);
    APP_MANAGER.get().exit_current();
    restore_to_kernel()
}

pub(crate) fn run_app() -> usize {
    RUNNING.store(true, core::sync::atomic::Ordering::Relaxed);
    let mut mgr = APP_MANAGER.get();
    unsafe { mgr.load_app() };
    let mut process = mgr.get_process().clone();
    drop(mgr);
    let mut current_pc= process.pc;
    let stack_index = process.stack.unwrap();
    let mut app_num = 0;
    let user_top = USER_STACK[stack_index].get_sp_top();
    loop {
        unsafe {
            arch_relate::run_app(process);
            // arch_relate::run_app(user_top, current_pc);
            app_num += 1;
        }
        let mut mgr = APP_MANAGER.get();
        if mgr.nxt_app() {
            // current_pc = mgr.get_process().pc;
            process = mgr.get_process().clone();
        }
        else {
            break;
        }
    }
    RUNNING.store(false, core::sync::atomic::Ordering::Relaxed);
    trace!("run_app trace");
    app_num
}

pub(crate) fn restore_to_kernel() -> ! {
    let mut mgr = APP_MANAGER.get();
    let ctx_ptr = core::ptr::addr_of_mut!(mgr.kernel_ctx);
    drop(mgr);
    unsafe {
        trap_restore(&mut (*ctx_ptr));
    };
}

pub(crate) fn write_task(id: *mut usize, name: *mut u8, len: usize) -> usize {
    let mgr = APP_MANAGER.get();
    let str_name = "hahaha";
    let min_len = min(str_name.len(), len);
    unsafe {
        name.copy_from(str_name.as_ptr(), min_len);
        if min_len < len {
            name.add(min_len).write_volatile(b'\0');
        }
        id.write_volatile(mgr.current_app);
    };
    min_len
}
