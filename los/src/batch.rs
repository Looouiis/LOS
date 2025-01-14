use core::{
    cell::{RefCell, RefMut},
    cmp::min,
};
use alloc::collections::linked_list::LinkedList;
use lazy_static::lazy_static;
use spin::mutex::Mutex;

use crate::{
    arch_relate::{
        self,
        timer::set_nxt_trigger,
        trap::{trap_return, TrapContext},
    }, config::TRAP_CONTEXT, mem::{address::{PhyPageNum, VirAddr}, memory_set::{MemorySet, KERNEL_SPACE}}, stack::{KERNAL_STACK_SIZE, TRAP_STACK}
};

extern "C" {
    fn _num_program();
}

// pub(crate) const MAX_PROGRAM_NUM: usize = 20;

pub(crate) enum RestoreBehavior {
    DirectReturn(usize),
    Reschedule,
}

lazy_static! {
    pub(crate) static ref PROGRAM_MANAGER: ArcCell<ProgramManager> = unsafe {
        ArcCell::new({
            let ptr = _num_program as usize as *const usize;
            let program_num = ptr.read_volatile();
            // let process: [Process; MAX_PROGRAM_NUM + 1] = [Process {
            //     // pc: 0,
            //     // start: 0,
            //     // len: 0,
            //     status: State::Exited,
            //     stack: None,
            //     ctx: TrapContext::new(),
            // }; MAX_PROGRAM_NUM + 1];
            ProgramManager {
                program_num,
                current_program: Mutex::new(None),
                process: LinkedList::new(),
                kernel_ctx: TrapContext::new(),
                // current_entry: ProgramManager::ENTRY,
            }
        })
    };
}

pub(crate) struct Process {
    pub(crate) pid: usize,
    // pub(crate) start: usize,
    // pub(crate) pc: usize,
    // pub(crate) len: usize,
    pub(crate) status: State,
    // stack: Option<UserStack>,
    pub(crate) stack: Option<usize>,
    // pub(crate) ctx: TrapContext,
    pub(crate) memory_set: MemorySet,
    pub(crate) trapctx_ppn: PhyPageNum,
    pub(crate) base_size: usize
}

impl Process {
    pub(crate) fn get_trap_context(&self) -> &'static mut TrapContext {
        self.trapctx_ppn.get_mut_data_at_start()
    }

    pub(crate) fn get_satp(&self) -> usize {
        arch_relate::to_satp(self.memory_set.page_table.address())
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum State {
    Ready,
    Exited,
}

pub(crate) struct ProgramManager {
    program_num: usize,
    current_program: Mutex<Option<Process>>,
    process: LinkedList<Process>,
    pub(crate) kernel_ctx: TrapContext,
    // current_entry: usize,
}

impl ProgramManager {
    pub(crate) const ENTRY: usize = 0x80400000;
    pub(crate) const LIMIT: usize = 0x20000;

    pub fn print_info(&self) {
        log!("Program_num: {}", self.program_num);
    }

    pub(crate) fn init(&mut self) {
        extern "C" {
            fn __trampoline_start();
        }
        let kernel_satp = arch_relate::to_satp(KERNEL_SPACE.get().page_table.address());
        for i in 0 .. self.program_num {
            let (memory_set, user_sp_top_va, entry_point) = MemorySet::from_elf(unsafe { self.get_program_elf_bytes(i) });
            let trapctx_ppn = memory_set.page_table.vpn_to_pte(VirAddr::from(TRAP_CONTEXT).floor_to_vpn()).unwrap().ppn();
            let ctx: &mut TrapContext = trapctx_ppn.get_mut_data_at_start();
            // !ctx.sepc在arch_relate中的run_program中确定
            ctx.sepc = entry_point;
            // ctx.info.sp = USER_STACK[i].get_sp_top();
            ctx.info.sp = user_sp_top_va;
            ctx.kernel_satp = kernel_satp;
            ctx.trap_handler = arch_relate::syscall_handler::syscall_service as usize;
            ctx.kernel_sp = core::ptr::addr_of!(TRAP_STACK) as usize + KERNAL_STACK_SIZE;
            let process = Process {
                pid: i,
                status: State::Ready,
                stack: Some(i),
                memory_set,
                trapctx_ppn,
                base_size: user_sp_top_va,
            };
            self.process.push_back(process);
        }
        let first = self.process.pop_front();
        *self.current_program.lock() = first;
    }

    unsafe fn get_program_elf_bytes(&mut self, id: usize) -> &'static [u8] {
        // let ptr = _num_program as usize as *const usize;
        // let program_num = ptr.read_volatile();
        // for i in 0..=program_num {
        //     let start = ptr.add(1 + i).read_volatile();
        //     self.process[i].start = start;
        //     self.process[i].ctx.sepc = start;
        //     self.process[i].len = ProgramManager::LIMIT;
        //     self.process[i].status = State::Ready;
        //     self.process[i].stack = Some(i);
        //     self.process[i].ctx.info.sp = USER_STACK[i].get_sp_top();
        // }
        // (0..self.program_num).for_each(|id| {
        //     let mut ptr = (Self::ENTRY + id * Self::LIMIT) as *mut u8;
        //     (self.process[id].start..self.process[id].start + self.process[id].len).for_each(
        //         |raw| {
        //             let ch = (raw as *mut u8).read_volatile();
        //             ptr.write_volatile(ch);
        //             ptr = ptr.add(1);
        //         },
        //     );
        //     fence!();
        // });
        let ptr = _num_program as usize as *const usize;
        let program_start_ptr = core::slice::from_raw_parts(ptr.add(1), self.program_num + 1);
        assert!(id < self.program_num);
        core::slice::from_raw_parts(program_start_ptr[id] as *const u8, program_start_ptr[id + 1] - program_start_ptr[id])
    }

    // 将program_manager内部的指针转移指向下一个program
    fn nxt_program(&mut self) -> bool {
        // let mut index = 1;
        // while index <= self.program_num {
        //     if self.process.get(&((self.current_program + index) % self.program_num)).unwrap().status
        //         == State::Ready
        //     {
        //         break;
        //     }
        //     index += 1;
        // }
        // if index == self.program_num + 1 {
        //     log!("Execute complete");
        //     false
        // } else {
        //     self.current_program = (self.current_program + index) % self.program_num;
        //     // self.current_entry = Self::ENTRY + Self::LIMIT * self.current_program;
        //     true
        // }
        match self.process.pop_front() {
            Some(p) => {
                let mut guard = self.current_program.lock();
                match guard.take() {
                    Some(cur_process) => {
                        self.process.push_back(cur_process);
                    },
                    None => {},
                }
                *guard = Some(p);
                true
            },
            None => false,
        }
    }

    pub(crate) fn get_process(&self) -> &Mutex<Option<Process>> {
        &self.current_program
    }

    pub(crate) fn exit_current(&mut self) {
        let mut exited = self.current_program.lock().take().unwrap();
        exited.status = State::Exited;
        drop(exited);
    }

    pub(crate) fn get_current_satp(&self) -> usize {
        self.current_program.lock().as_ref().unwrap().get_satp()
    }

    pub(crate) fn get_current_trap_context(&self) -> &'static mut TrapContext {
        self.current_program.lock().as_ref().unwrap().get_trap_context()
    }
}

unsafe impl<T> Sync for ArcCell<T> {}

pub(crate) struct ArcCell<T> {
    inner: RefCell<T>,
}

impl<T> ArcCell<T> {
    pub(crate) fn new(item: T) -> Self {
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
pub(crate) fn reschedule() -> ! {
    set_nxt_trigger();
    restore_to_kernel()
}

#[inline]
pub(crate) fn sys_yield() -> RestoreBehavior {
    RestoreBehavior::Reschedule
}

pub(crate) fn run_program() -> usize {
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
    trap_return(true);
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
        id.write_volatile(mgr.current_program.lock().as_ref().unwrap().pid);
    };
    RestoreBehavior::DirectReturn(min_len)
}
