use core::cell::{RefCell, RefMut};

use lazy_static::lazy_static;

use crate::{arch_relate::{self, syscall_handler::trap::{trap_restore, TrapContext}}, stack::USER_STACK};

extern "C" {
    fn _num_app();
}

const MAX_APP_NUM: usize = 10;

lazy_static!{
    pub(crate) static ref APP_MANAGER: ArcCell<AppManager> = unsafe {
        ArcCell::new({
            let ptr = _num_app as usize as *const usize;
            let app_num = ptr.read_volatile();
            let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
            let start_slice = core::slice::from_raw_parts(ptr.add(1), app_num + 1);
            app_start[..= app_num].copy_from_slice(start_slice);
            AppManager {
                app_num,
                current_app: 0,
                app_start,
                kernel_ctx: TrapContext::new()
            }
        })
    };
}

pub(crate) struct AppManager {
    app_num: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],
    pub(crate) kernel_ctx: TrapContext
}

impl AppManager {
    pub(crate) const ENTRY: usize = 0x80400000;

    pub fn print_info(&self) {
        log!("app_num: {}", self.app_num);
    }

    unsafe fn load_app(&self) {
        let mut ptr = Self::ENTRY as *mut u8;
        (self.app_start[self.current_app] .. self.app_start[self.current_app + 1]).for_each(|raw| {
            let ch = (raw as *mut u8).read_volatile();
            ptr.write_volatile(ch);
            ptr = ptr.add(1);
        });
        fence!();
    }

    fn nxt_app(&mut self) -> bool {
        self.current_app = self.current_app + 1;
        if self.app_num == self.current_app {
            log!("Execute complete");
            false
        }
        else {
            true
        }
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
    restore_to_kernel()
}

#[no_mangle]
pub(crate) fn run_app() -> usize {
    let mgr = APP_MANAGER.get();
    unsafe { mgr.load_app() };
    drop(mgr);
    let mut app_num = 0;
    let user_top = USER_STACK.get_sp_top();
    loop {
        unsafe {
            arch_relate::run_app(user_top);
            app_num += 1;
        }
        let mut mgr = APP_MANAGER.get();
        if mgr.nxt_app() {
            unsafe { mgr.load_app() };
        }
        else {
            break;
        }
    }
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
