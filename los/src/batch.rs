use core::{
    cell::{RefCell, RefMut},
    cmp::min,
};
use lazy_static::lazy_static;

use crate::{
    arch_relate::{
        self, timer::set_nxt_trigger, trap::{trap_restore, TrapContext}
    },
    stack::USER_STACK,
};

extern "C" {
    fn _num_program();
}

pub(crate) const MAX_PROGRAM_NUM: usize = 20;

pub(crate) enum RestoreBehavior {
    DirectReturen(usize),
    Reschedule,
}

lazy_static! {
    pub(crate) static ref PROGRAM_MANAGER: ArcCell<ProgramManager> = unsafe {
        ArcCell::new({
            let ptr = _num_program as usize as *const usize;
            let program_num = ptr.read_volatile();
            let process: [Process; MAX_PROGRAM_NUM + 1] = [Process {
                pc: 0,
                start: 0,
                len: 0,
                status: State::Exited,
                stack: None,
                ctx: TrapContext::new(),
            }; MAX_PROGRAM_NUM + 1];
            ProgramManager {
                program_num,
                current_program: 0,
                process,
                kernel_ctx: TrapContext::new(),
                current_entry: ProgramManager::ENTRY,
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
pub(crate) enum State {
    Ready,
    Exited,
}

pub(crate) struct ProgramManager {
    program_num: usize,
    current_program: usize,
    process: [Process; MAX_PROGRAM_NUM + 1],
    pub(crate) kernel_ctx: TrapContext,
    current_entry: usize,
}

impl ProgramManager {
    pub(crate) const ENTRY: usize = 0x80400000;
    pub(crate) const LIMIT: usize = 0x20000;

    pub fn print_info(&self) {
        log!("Program_num: {}", self.program_num);
    }

    unsafe fn load_program(&mut self) {
        let ptr = _num_program as usize as *const usize;
        let program_num = ptr.read_volatile();
        for i in 0..=program_num {
            let start = ptr.add(1 + i).read_volatile();
            self.process[i].start = start;
            self.process[i].pc = start;
            self.process[i].len = ProgramManager::LIMIT;
            self.process[i].status = State::Ready;
            self.process[i].stack = Some(i);
            self.process[i].ctx.info.sp = USER_STACK[i].get_sp_top();
        }
        (0..self.program_num).for_each(|id| {
            let mut ptr = (Self::ENTRY + id * Self::LIMIT) as *mut u8;
            (self.process[id].start..self.process[id].start + self.process[id].len).for_each(
                |raw| {
                    let ch = (raw as *mut u8).read_volatile();
                    ptr.write_volatile(ch);
                    ptr = ptr.add(1);
                },
            );
            fence!();
        });
    }

    // 将program_manager内部的指针转移指向下一个program
    fn nxt_program(&mut self) -> bool {
        let mut index = 1;
        while index <= self.program_num {
            if self.process[(self.current_program + index) % self.program_num].status
                == State::Ready
            {
                break;
            }
            index += 1;
        }
        if index == self.program_num + 1 {
            log!("Execute complete");
            false
        } else {
            self.current_program = (self.current_program + index) % self.program_num;
            self.current_entry = Self::ENTRY + Self::LIMIT * self.current_program;
            true
        }
    }

    pub(crate) fn get_process(&self) -> &Process {
        &self.process[self.current_program]
    }

    pub(crate) fn exit_current(&mut self) {
        self.process[self.current_program].status = State::Exited;
    }

    pub(crate) fn mark_pc(&mut self, pc: usize) {
        self.process[self.current_program].pc = pc;
    }

    pub(crate) fn mark_ctx(&mut self, ctx: TrapContext) {
        self.process[self.current_program].ctx = ctx;
    }
}

unsafe impl<T> Sync for ArcCell<T> {}

pub(crate) struct ArcCell<T> {
    inner: RefCell<T>,
}

impl<T> ArcCell<T> {
    fn new(item: T) -> Self {
        Self {
            inner: RefCell::new(item),
        }
    }

    pub fn get(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}

#[inline]
pub(crate) fn exit(code: usize) -> ! {
    log!("Program exit with {}", code);
    PROGRAM_MANAGER.get().exit_current();
    restore_to_kernel()
}

#[inline]
pub(crate) fn reschedule(ctx: TrapContext) -> ! {
    PROGRAM_MANAGER.get().mark_ctx(ctx);
    set_nxt_trigger();
    restore_to_kernel()
}

#[inline]
pub(crate) fn sys_yield() -> RestoreBehavior {
    RestoreBehavior::Reschedule
}

pub(crate) fn run_program() -> usize {
    let mut mgr = PROGRAM_MANAGER.get();
    unsafe { mgr.load_program() };
    drop(mgr);
    let mut entered_num = 0;
    loop {
        unsafe {
            arch_relate::run_program();
            entered_num += 1;
        }
        let mut mgr = PROGRAM_MANAGER.get();
        if !mgr.nxt_program() {
            break;
        }
    }
    trace!("run_program trace");
    entered_num
}

#[inline]
pub(crate) fn restore_to_kernel() -> ! {
    let mut mgr = PROGRAM_MANAGER.get();
    let ctx_ptr = core::ptr::addr_of_mut!(mgr.kernel_ctx);
    drop(mgr);
    unsafe {
        trap_restore(&mut (*ctx_ptr));
    };
}

pub(crate) fn write_task(id: *mut usize, name: *mut u8, len: usize) -> RestoreBehavior {
    let mgr = PROGRAM_MANAGER.get();
    let str_name = "hahaha";
    let min_len = min(str_name.len(), len);
    unsafe {
        name.copy_from(str_name.as_ptr(), min_len);
        if min_len < len {
            name.add(min_len).write_volatile(b'\0');
        }
        id.write_volatile(mgr.current_program);
    };
    RestoreBehavior::DirectReturen(min_len)
}
