use alloc::{
    collections::{btree_map::BTreeMap, linked_list::LinkedList},
    string::String,
};
use core::{
    cell::{RefCell, RefMut},
    cmp::min,
};
use lazy_static::lazy_static;
use spin::mutex::Mutex;

use crate::{
    arch_relate::{
        self,
        timer::set_nxt_trigger,
        trap::{trap_return, TrapContext},
    },
    config::TRAP_CONTEXT,
    mem::{
        address::{PhyPageNum, VirAddr},
        memory_set::{MemorySet, KERNEL_SPACE},
    },
    stack::{KERNAL_STACK_SIZE, TRAP_STACK},
};

pub(crate) mod syscall_fn;

extern "C" {
    fn _num_program();
    fn __trampoline_start();
    fn _program_names();
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
            ProgramManager {
                program_num,
                current_program: Mutex::new(None),
                process: LinkedList::new(),
                kernel_ctx: TrapContext::new(),
                name_map: BTreeMap::new(),
            }
        })
    };
}

pub(crate) struct Process {
    pub(crate) pid: usize,
    pub(crate) status: State,
    pub(crate) memory_set: MemorySet,
    pub(crate) trapctx_ppn: PhyPageNum,
}

impl Process {
    pub(crate) fn get_trap_context(&self) -> &'static mut TrapContext {
        self.trapctx_ppn.get_mut_data_at_start()
    }

    pub(crate) fn get_token(&self) -> usize {
        arch_relate::to_token(self.memory_set.page_table.address())
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
    name_map: BTreeMap<String, usize>,
    // current_entry: usize,
}

impl ProgramManager {
    pub fn print_info(&self) {
        log!("Program_num: {}", self.program_num);
    }

    pub(crate) fn init(&mut self) {
        // 读取名字
        let mut name_ptr = _program_names as usize as *const u8;
        unsafe {
            // name_ptr = name_ptr.add(1);
        }
        let mut str = String::new();
        for i in 0..self.program_num {
            unsafe {
                let mut ch = name_ptr.read_volatile();
                while ch != b'\0' {
                    str.push(ch as char);
                    name_ptr = name_ptr.add(1);
                    ch = name_ptr.read_volatile();
                }
                name_ptr = name_ptr.add(1);
            }
            println!("{str}");
            self.name_map.insert(str.clone(), i);
            str.clear();
        }
        println!("{:?}", self.name_map);
        // 进程相关
        let kernel_satp = arch_relate::to_token(KERNEL_SPACE.get().page_table.address());
        for i in 0..self.program_num {
            let (memory_set, user_sp_top_va, entry_point) =
                MemorySet::from_elf(unsafe { self.get_program_elf_bytes(i) });
            let trapctx_ppn = memory_set
                .page_table
                .vpn_to_pte(VirAddr::from(TRAP_CONTEXT).floor_to_vpn())
                .unwrap()
                .ppn();
            let ctx: &mut TrapContext = trapctx_ppn.get_mut_data_at_start();
            ctx.sepc = entry_point;
            ctx.info.sp = user_sp_top_va;
            ctx.kernel_satp = kernel_satp;
            ctx.trap_handler = arch_relate::syscall_handler::syscall_service as usize;
            ctx.kernel_sp = core::ptr::addr_of!(TRAP_STACK) as usize + KERNAL_STACK_SIZE;
            let process = Process {
                pid: i,
                status: State::Ready,
                memory_set,
                trapctx_ppn,
            };
            self.process.push_back(process);
        }
        let first = self.process.pop_front();
        *self.current_program.lock() = first;
    }

    unsafe fn get_program_elf_bytes(&mut self, id: usize) -> &'static [u8] {
        let ptr = _num_program as usize as *const usize;
        let program_start_ptr = core::slice::from_raw_parts(ptr.add(1), self.program_num + 1);
        assert!(id < self.program_num);
        core::slice::from_raw_parts(
            program_start_ptr[id] as *const u8,
            program_start_ptr[id + 1] - program_start_ptr[id],
        )
    }

    // 将program_manager内部的指针转移指向下一个program
    fn nxt_program(&mut self) -> bool {
        match self.process.pop_front() {
            Some(p) => {
                let mut guard = self.current_program.lock();
                match guard.take() {
                    Some(cur_process) => {
                        self.process.push_back(cur_process);
                    }
                    None => {}
                }
                *guard = Some(p);
                true
            }
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

    pub(crate) fn get_current_token(&self) -> usize {
        self.current_program.lock().as_ref().unwrap().get_token()
    }

    pub(crate) fn get_current_trap_context(&self) -> &'static mut TrapContext {
        self.current_program
            .lock()
            .as_ref()
            .unwrap()
            .get_trap_context()
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
        match self.inner.try_borrow_mut() {
            Ok(res) => res,
            Err(e) => {
                panic!("{e:?}")
            }
        }
    }
}

impl<T> Drop for ArcCell<T> {
    fn drop(&mut self) {
        log!("In release mode, drop fn can't be opt out");
    }
}

#[inline]
pub(crate) fn reschedule() -> ! {
    set_nxt_trigger();
    restore_to_kernel()
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
        drop(mgr);
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
