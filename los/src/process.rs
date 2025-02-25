use alloc::{
    alloc::{alloc, dealloc},
    collections::linked_list::LinkedList,
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};
use core::{
    alloc::Layout,
    cell::{RefCell, RefMut},
    cmp::min,
    ops::Deref,
};
use lazy_static::lazy_static;
use spin::mutex::Mutex;
use syscall_spec::fs::FileFlags;

use crate::{
    arch_relate::{
        self, switch,
        timer::set_nxt_trigger,
        trap::{trap_return, ProcessContext, TrapContext},
    },
    config::TRAP_CONTEXT,
    fs::{open_file, File, ROOT_INODE},
    io::Stdio,
    mem::{
        address::{PhyPageNum, VirAddr},
        memory_set::{MemorySet, KERNEL_SPACE},
        page_table::ROTable,
    },
    stack::{KernelStack, KERNAL_STACK_SIZE},
};

pub(crate) mod syscall_fn;

extern "C" {
    fn _num_program();
    fn __trampoline_start();
    fn _program_names();
}

pub(crate) enum RestoreBehavior {
    DirectReturn(usize),
    Reschedule,
}

lazy_static! {
    pub(crate) static ref PROCESS_MANAGER: ArcCell<ProcessManager> = ArcCell::new({
        ProcessManager {
            current_program: None,
            process: LinkedList::new(),
            kernel_ctx: ProcessContext::new(),
            kernel_token: 0,
        }
    });
}

pub(crate) static PID_ALLOCATOR: Mutex<PidAllocator> = Mutex::new(PidAllocator::new());

#[derive(PartialEq, PartialOrd, Clone)]
pub(crate) struct PidWrapper(usize);

impl Drop for PidWrapper {
    fn drop(&mut self) {
        PID_ALLOCATOR.lock().dealloc(self.0);
    }
}

impl From<&PidWrapper> for isize {
    fn from(value: &PidWrapper) -> Self {
        value.0 as isize
    }
}

impl From<&PidWrapper> for usize {
    fn from(value: &PidWrapper) -> Self {
        value.0
    }
}

impl Deref for PidWrapper {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) struct PidAllocator {
    start: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    pub(crate) const fn new() -> Self {
        Self {
            start: 0,
            end: usize::MAX - 1, // usize::MAX（即-1）被waitpid使用了
            recycled: Vec::new(),
        }
    }

    pub(crate) fn alloc(&mut self) -> Option<PidWrapper> {
        if !self.recycled.is_empty() {
            let id = self.recycled.pop().unwrap();
            return Some(PidWrapper(id));
        } else {
            if self.start < self.end {
                let res = self.start;
                self.start += 1;
                return Some(PidWrapper(res));
            } else {
                return None;
            }
        }
    }

    pub(crate) fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.start);
        self.recycled.push(pid);
    }
}

pub(crate) struct Process {
    pub(crate) pid: PidWrapper,
    pub(crate) status: State,
    pub(crate) memory_set: MemorySet,
    pub(crate) trapctx_ppn: PhyPageNum,
    pub(crate) process_ctx: ProcessContext,
    pub(crate) children: Vec<Weak<Mutex<Process>>>,
    #[allow(unused)]
    pub(crate) father: Option<Weak<Mutex<Process>>>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
}

impl Process {
    pub(crate) fn get_trap_context(&self) -> &'static mut TrapContext {
        self.trapctx_ppn.get_mut_data_at_start()
    }

    pub(crate) fn get_token(&self) -> usize {
        arch_relate::to_token(self.memory_set.page_table.address())
    }

    pub(crate) fn new(elf_data: &[u8]) -> Arc<Mutex<Self>> {
        let kernel_token = arch_relate::to_token(KERNEL_SPACE.get().page_table.address());
        let (memory_set, user_sp_top_va, entry_point) = MemorySet::from_elf(elf_data);
        let trapctx_ppn = memory_set
            .page_table
            .vpn_to_pte(VirAddr::from(TRAP_CONTEXT).floor_to_vpn())
            .unwrap()
            .ppn();
        let ctx: &mut TrapContext = trapctx_ppn.get_mut_data_at_start();
        ctx.sepc = entry_point;
        ctx.info.sp = user_sp_top_va;
        ctx.kernel_token = kernel_token;
        ctx.trap_handler = arch_relate::syscall_handler::syscall_service as usize;
        let kernel_stack =
            unsafe { alloc(Layout::from_size_align(size_of::<KernelStack>(), 4096).unwrap()) };
        ctx.kernel_sp = kernel_stack as usize + KERNAL_STACK_SIZE;
        let mut process_ctx = ProcessContext::new();
        process_ctx.init(trap_return as usize, ctx.kernel_sp);
        let stdio = Arc::new(Stdio);
        Arc::new(Mutex::new(Self {
            pid: PID_ALLOCATOR.lock().alloc().unwrap(),
            status: State::Ready,
            memory_set,
            trapctx_ppn,
            process_ctx,
            children: Vec::new(),
            father: None,
            exit_code: None,
            fd_table: vec![Some(stdio.clone()), Some(stdio.clone())],
        }))
    }

    pub(crate) fn exec(&mut self, elf_data: &[u8]) {
        let kernel_stack_top = self
            .memory_set
            .page_table
            .vpn_to_pte(VirAddr::from(TRAP_CONTEXT).floor_to_vpn())
            .unwrap()
            .ppn()
            .get_mut_data_at_start::<TrapContext>()
            .kernel_sp;
        let kernel_token = arch_relate::to_token(KERNEL_SPACE.get().page_table.address());
        let (memory_set, user_sp_top_va, entry_point) = MemorySet::from_elf(elf_data);
        let trapctx_ppn = memory_set
            .page_table
            .vpn_to_pte(VirAddr::from(TRAP_CONTEXT).floor_to_vpn())
            .unwrap()
            .ppn();
        let ctx: &mut TrapContext = trapctx_ppn.get_mut_data_at_start();
        ctx.sepc = entry_point;
        ctx.info.sp = user_sp_top_va;
        ctx.kernel_token = kernel_token;
        ctx.trap_handler = arch_relate::syscall_handler::syscall_service as usize;
        ctx.kernel_sp = kernel_stack_top;
        self.status = State::Ready;
        self.memory_set = memory_set;
        self.trapctx_ppn = trapctx_ppn;
    }

    pub(crate) fn fork(from: &Arc<Mutex<Self>>) -> Arc<Mutex<Self>> {
        let mut guard = from.lock();
        let mut memory_set = MemorySet::empty();
        memory_set.fork_from(&guard.memory_set);
        memory_set.map_trampoline();
        let src = &mut (guard.trapctx_ppn.get_bytes_array()[..size_of::<TrapContext>()]);
        let trapctx_ppn = memory_set
            .page_table
            .vpn_to_pte(VirAddr::from(TRAP_CONTEXT).floor_to_vpn())
            .unwrap()
            .ppn();
        let dst = &mut (trapctx_ppn.get_bytes_array()[..src.len()]);
        dst.copy_from_slice(&src);
        let ctx: &mut TrapContext = trapctx_ppn.get_mut_data_at_start();
        let kernel_stack =
            unsafe { alloc(Layout::from_size_align(size_of::<KernelStack>(), 4096).unwrap()) };
        ctx.kernel_sp = kernel_stack as usize + KERNAL_STACK_SIZE;
        ctx.info.a0 = 0;
        let mut process_ctx = ProcessContext::new();
        process_ctx.init(trap_return as usize, ctx.kernel_sp);
        let status = guard.status;
        let fd_table = guard.fd_table.iter().map(|file| file.clone()).collect();
        let res = Arc::new(Mutex::new(Self {
            pid: PID_ALLOCATOR.lock().alloc().unwrap(),
            status,
            memory_set,
            trapctx_ppn,
            process_ctx,
            children: Vec::new(),
            father: Some(Arc::downgrade(from)),
            exit_code: None,
            fd_table,
        }));
        guard.children.push(Arc::downgrade(&res));
        res
    }

    pub(crate) fn alloc_fd(&mut self) -> usize {
        (0..self.fd_table.len())
            .find(|idx| self.fd_table[*idx].is_none())
            .unwrap_or({
                self.fd_table.push(None);
                self.fd_table.len() - 1
            })
    }

    pub(crate) fn set_fd(&mut self, fd: usize, file: Arc<dyn File + Send + Sync>) {
        self.fd_table[fd] = Some(file)
    }

    pub(crate) fn dealloc_fd(&mut self, fd: usize) -> bool {
        if fd >= self.fd_table.len() || self.fd_table[fd].is_some() {
            false
        } else {
            self.fd_table[fd].take();
            true
        }
    }

    pub(crate) fn get_fd(&self, fd: usize) -> &Option<Arc<dyn File + Send + Sync>> {
        self.fd_table.get(fd).unwrap_or(&None)
    }
}

pub(crate) fn sys_waitpid(pid: isize, exit_code: *mut i32) -> RestoreBehavior {
    let mut mgr = PROCESS_MANAGER.get();
    let cur = mgr.current_program.as_ref().unwrap().clone();
    let mut cur_guard = cur.lock();
    let token = cur_guard.get_token();
    let page_table = ROTable::from_token(token);
    if cur_guard.children.is_empty() {
        return RestoreBehavior::DirectReturn(-1isize as usize);
    }
    match pid {
        -1 => {
            let mut find_res = None;
            for (index, process) in cur_guard.children.iter().enumerate() {
                let arc = process.upgrade().unwrap();
                let guard = arc.lock();
                if guard.status == State::Exited {
                    let pid = guard.pid.0;
                    drop(guard);
                    match mgr.drop_process(pid) {
                        Some(res) => {
                            find_res = Some((pid, index, res));
                        }
                        None => {
                            panic!("internal error");
                        }
                    }
                    break;
                }
            }
            match find_res {
                Some((pid, index, res)) => {
                    cur_guard.children.swap_remove(index);
                    if !exit_code.is_null() {
                        let va = VirAddr::from(exit_code as usize);
                        let ptr: usize = page_table.va_to_pa(va).unwrap().into();
                        unsafe { (ptr as *mut i32).write_volatile(res) };
                    }
                    return RestoreBehavior::DirectReturn(pid);
                }
                None => {
                    return RestoreBehavior::DirectReturn(-2isize as usize);
                }
            }
        }
        pid => {
            drop(mgr);
            for (index, process) in cur_guard.children.iter().enumerate() {
                let arc = process.upgrade().unwrap();
                let guard = arc.lock();
                let tar_pid = guard.pid.0;
                if tar_pid == pid as usize {
                    drop(guard);
                    loop {
                        let status = arc.lock().status;
                        if status != State::Exited {
                            drop(cur_guard);
                            switch_task();
                            cur_guard = cur.lock();
                        } else {
                            match PROCESS_MANAGER.get().drop_process(tar_pid) {
                                Some(res) => {
                                    if !exit_code.is_null() {
                                        let va = VirAddr::from(exit_code as usize);
                                        let ptr: usize = page_table.va_to_pa(va).unwrap().into();
                                        unsafe { (ptr as *mut i32).write_volatile(res) };
                                    }
                                    cur_guard.children.swap_remove(index);
                                    return RestoreBehavior::DirectReturn(tar_pid.into());
                                }
                                None => {
                                    panic!("internal error");
                                }
                            }
                        }
                    }
                }
            }
            return RestoreBehavior::DirectReturn(-1isize as usize);
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let kernel_sp_top = self
            .trapctx_ppn
            .get_mut_data_at_start::<TrapContext>()
            .kernel_sp;
        let kernel_sp_ptr = kernel_sp_top - KERNAL_STACK_SIZE;
        unsafe {
            dealloc(
                kernel_sp_ptr as *mut u8,
                Layout::from_size_align(size_of::<KernelStack>(), 4096).unwrap(),
            )
        };
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum State {
    Ready,
    Exited,
}

pub(crate) struct ProcessManager {
    current_program: Option<Arc<Mutex<Process>>>,
    process: LinkedList<Arc<Mutex<Process>>>,
    pub(crate) kernel_ctx: ProcessContext,
    // name_map: BTreeMap<String, usize>,
    kernel_token: usize,
}

impl ProcessManager {
    pub fn print_info(&self) {
        let programs = ROOT_INODE.ls();
        log!("___programs___");
        for program in programs {
            log!("{program}");
        }
    }

    pub(crate) fn init(&mut self) {
        let kernel_token = arch_relate::to_token(KERNEL_SPACE.get().page_table.address());
        self.kernel_token = kernel_token;
    }

    pub(crate) fn add_task(&mut self, name: &str) {
        match /* self.get_elf_by_name(name) */ open_file(name, FileFlags::RDONLY).map(|file| file.read_all()) {
            Some(data) => {
                let process = Process::new(data.as_slice());
                match self.current_program {
                    Some(_) => {
                        self.process.push_back(process);
                    }
                    None => {
                        self.current_program = Some(process);
                    }
                }
            }
            None => {
                panic!("internal error, name: {}", name);
            }
        }
    }

    // 将program_manager内部的指针转移指向下一个program
    fn nxt_program(&mut self) -> *const ProcessContext {
        for _i in 0..self.process.len() {
            match self.process.pop_front() {
                Some(p) => {
                    if p.lock().status != State::Ready {
                        self.process.push_back(p);
                        continue;
                    }
                    match &self.current_program {
                        Some(process) => {
                            self.process.push_back(process.clone());
                        }
                        None => {}
                    }
                    self.current_program = Some(p);
                    return &(self.current_program.as_ref().unwrap().lock().process_ctx)
                        as *const ProcessContext;
                }
                None => {}
            }
        }
        if self.current_program.is_some()
            && self.current_program.as_ref().unwrap().lock().status == State::Ready
        {
            return &(self.current_program.as_ref().unwrap().lock().process_ctx)
                as *const ProcessContext;
        } else {
            let res = &self.kernel_ctx as *const ProcessContext;
            return res;
        }
    }

    pub(crate) fn get_process(&self) -> &Option<Arc<Mutex<Process>>> {
        &self.current_program
    }

    pub(crate) fn exit_current(&mut self, exit_code: i32) {
        let exited = self.current_program.as_mut().unwrap();
        let mut guard = exited.lock();
        guard.status = State::Exited;
        if guard.pid.0 != 0 {
            let initproc = self
                .process
                .iter()
                .find(|item| match item.try_lock() {
                    Some(guard) => return guard.pid.0 == 0,
                    None => return false,
                })
                .unwrap();
            initproc.lock().children.append(&mut guard.children);
        }
        guard.exit_code = Some(exit_code);
    }

    pub(crate) fn get_current_token(&self) -> usize {
        self.current_program.as_ref().unwrap().lock().get_token()
    }

    pub(crate) fn get_current_trap_context(&self) -> &'static mut TrapContext {
        self.current_program
            .as_ref()
            .unwrap()
            .lock()
            .get_trap_context()
    }

    pub(crate) fn drop_process(&mut self, tar_pid: usize) -> Option<i32> {
        for _i in 0..self.process.len() {
            let process = self.process.pop_front().unwrap();
            let guard = process.lock();
            if guard.pid.0 == tar_pid {
                return guard.exit_code;
            } else {
                drop(guard);
                self.process.push_back(process);
            }
        }
        None
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
pub(crate) fn reschedule() {
    set_nxt_trigger();
    switch_task();
}

pub(crate) fn run_program() -> usize {
    let mut entered_num = 0;
    unsafe {
        arch_relate::run_program();
        entered_num += 1;
    }
    trace!("run_program trace");
    entered_num
}

#[inline]
pub(crate) fn switch_task() {
    let mut mgr = PROCESS_MANAGER.get();
    let guard = mgr.current_program.as_ref().unwrap().lock();
    let cur_ctx = core::ptr::addr_of!(guard.process_ctx);
    drop(guard);
    let nxt_ctx = mgr.nxt_program();
    assert!(!mgr.current_program.as_ref().unwrap().is_locked());
    drop(mgr);
    unsafe { switch(cur_ctx, nxt_ctx) };
}

pub(crate) fn write_task(id: *mut usize, name: *mut u8, len: usize) -> RestoreBehavior {
    let mgr = PROCESS_MANAGER.get();
    let str_name = "hahaha";
    let min_len = min(str_name.len(), len);
    unsafe {
        name.copy_from(str_name.as_ptr(), min_len);
        if min_len < len {
            name.add(min_len).write_volatile(b'\0');
        }
        id.write_volatile(*mgr.current_program.as_ref().unwrap().lock().pid);
    };
    RestoreBehavior::DirectReturn(min_len)
}
