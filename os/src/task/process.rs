use crate::fs::{File, Stdin, Stdout};
use crate::mm::{translated_refmut, MemorySet, KERNEL_SPACE};
use crate::sync::UpSafeCell;
use crate::task::id::{pid_alloc, PidHandle, RecycleAllocator};
use crate::task::task::TaskControlBlock;
use crate::task::{add_task, insert_into_pid2process, SignalFlags};
use crate::trap::{trap_handler, TrapContext};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefMut;

///
/// 进程控制块实际内容
///
/// @author: tryte
///
/// @date: 2026/5/20
pub struct ProcessControlBlockInner {
    /// 是否处于僵尸态
    pub is_zombie: bool,
    /// 进程内容所在内存区域
    pub memory_set: MemorySet,
    /// 父进程
    pub parent: Option<Weak<ProcessControlBlock>>,
    /// 子进程
    pub children: Vec<Arc<ProcessControlBlock>>,
    /// 退出码
    pub exit_code: i32,
    /// 持有的文件描述符列表
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    /// 接收到的信号
    pub signals: SignalFlags,
    /// 线程
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    /// 线程ID分配器
    pub tasks_res_allocator: RecycleAllocator,
}

impl ProcessControlBlockInner {
    #[allow(unused)]
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    pub fn alloc_fd(&mut self) -> usize {
        // 查找空文件描述符
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            // 新增空文件描述符并返回
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }

    ///
    /// 分配线程ID
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/20
    pub fn alloc_tid(&mut self) -> usize {
        self.tasks_res_allocator.alloc()
    }

    ///
    /// 回收线程ID
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/20
    pub fn dealloc_tid(&mut self, tid: usize) {
        self.tasks_res_allocator.dealloc(tid)
    }

    pub fn thread_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn get_task(&self, tid: usize) -> Arc<TaskControlBlock> {
        self.tasks[tid].as_ref().unwrap().clone()
    }
}

///
/// 进程控制块
///
/// @author: tryte
///
/// @date: 2026/5/20
pub struct ProcessControlBlock {
    pub pid: PidHandle,
    inner: UpSafeCell<ProcessControlBlockInner>,
}

impl ProcessControlBlock {
    ///
    /// 创建进程控制器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/30
    pub fn new(elf_data: &[u8]) -> Arc<Self> {
        // 获取进程内存区域集合
        let (memory_set, user_stack_base, entry_point) = MemorySet::from_elf(elf_data);

        // 分配进程id
        let pid_handle = pid_alloc();

        // 创建进程控制器
        let process = Arc::new(Self {
            pid: pid_handle,
            inner: unsafe {
                UpSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: vec![
                        // 0 -> stdin
                        Some(Arc::new(Stdin)),
                        // 1 -> stdout
                        Some(Arc::new(Stdout)),
                        // 2 -> stderr
                        Some(Arc::new(Stdout)),
                    ],
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    tasks_res_allocator: RecycleAllocator::new(),
                })
            },
        });

        // 创建主线程
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&process),
            user_stack_base,
            true,
        ));

        let task_inner = task.inner_exclusive_access();
        // 创建“陷入”上下文，这里看起来是在直接操作物理地址，但是因为在内核态的情况下（已经使用了内核页表的 MMU 设置）
        // 页表的虚拟内存地址使用的恒等映射，所以这里还是使用虚拟内存地址访问
        let trap_cx = task_inner.get_trap_cx();
        let user_stack_top = task_inner.res.as_ref().unwrap().user_stack_top();
        let kernel_stack_top = task.kernel_stack.get_top();
        drop(task_inner);

        // 将 TrapContext 的全部内容移动到栈中，*trap_cx = TrapContext 相当于 memcpy(sp, &cx)
        // 这个时候的 memcpy 操作/指针内容写入 操作是遵循内存写入规则（从低到高），因此 sp 指向的是 cx 结构体的起始位置，如下：
        //       high addr - boot_stack_lower_bound = 8kb
        // |-------------------| 栈底
        // |       sepc        | -- 第34个地址，偏移量 33 * 8（x0 的偏移量是0）
        // |       ....        |
        // |      sstatus      | --> cx 内容
        // |       ....        |
        // |        x1         |
        // |        x0         |
        // |-------------------| --> sp 栈顶
        // |                   |
        // |                   |
        // |                   | boot_stack_lower_bound 栈的下限位置
        //       lower addr
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_stack_top,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as *const () as usize,
        );

        // 将主线程添加进进程
        let mut process_inner = process.inner_exclusive_access();
        process_inner.tasks.push(Some(Arc::clone(&task)));
        drop(process_inner);
        insert_into_pid2process(process.getpid(), Arc::clone(&process));
        // 将线程加入线程管理器
        add_task(task);
        process
    }

    pub fn inner_exclusive_access(&self) -> RefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }

    ///
    /// 获取当前进程id
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/6
    pub fn getpid(&self) -> usize {
        self.pid.0
    }

    ///
    /// 创建子进程
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/7
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        // 取出当前进程控制块
        let mut parent = self.inner_exclusive_access();
        assert_eq!(parent.thread_count(), 1);
        // 创建新进程的内存描述集合并复制当前进程的内容
        let memory_set = MemorySet::from_existed_user(&parent.memory_set);
        // 分配进程ID
        let pid = pid_alloc();
        // 复制打开的文件描述符列表
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }
        // 将子进程添加到父进程
        let child = Arc::new(Self {
            pid,
            inner: unsafe {
                UpSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: new_fd_table,
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    tasks_res_allocator: RecycleAllocator::new(),
                })
            },
        });
        parent.children.push(Arc::clone(&child));

        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&child),
            parent
                .get_task(0)
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .user_stack_base(),
            // here we do not allocate trap_cx or ustack again
            // but mention that we allocate a new kstack here
            false,
        ));
        let mut child_inner = child.inner_exclusive_access();
        child_inner.tasks.push(Some(Arc::clone(&task)));
        drop(child_inner);

        let task_inner = task.inner_exclusive_access();
        // 设置新进程的内核栈顶为“陷入”上下文地址
        let trap_cx = task_inner.get_trap_cx();
        trap_cx.kernel_sp = task.kernel_stack.get_top();
        drop(task_inner);
        insert_into_pid2process(child.getpid(), Arc::clone(&child));
        add_task(task);
        child
    }

    ///
    /// 执行新程序
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/7
    pub fn exec(self: &Arc<Self>, elf_data: &[u8], args: Vec<String>) {
        assert_eq!(self.inner_exclusive_access().thread_count(), 1);

        // 创建新的内存区域描述集合
        let (memory_set, user_stack_base, entry_point) = MemorySet::from_elf(elf_data);
        let new_token = memory_set.token();
        self.inner_exclusive_access().memory_set = memory_set;

        // 将当前进程的内容替换成新进程内容
        let task = self.inner_exclusive_access().get_task(0);
        let mut task_inner = task.inner_exclusive_access();
        task_inner.res.as_mut().unwrap().user_stack_base = user_stack_base;
        task_inner.res.as_mut().unwrap().alloc_user_res();
        task_inner.trap_cx_ppn = task_inner.res.as_mut().unwrap().trap_cx_ppn();

        // 传入启动参数，放入进程的用户栈，最终布局
        // 高地址
        // ──────────────────
        // argv[3] = NULL
        // argv[2]
        // argv[1]
        // argv[0]
        // ──────────────────
        // "user_program\0"
        // "arg1\0"
        // "arg2\0"
        // 空洞
        // ──────────────────
        // 低地址
        // 根据 C ABI 要求，
        //  1. argv数组最后一个元素是NULL，因此argv数组的长度是 args.len() + 1
        //  2. 栈内数据需要保证 8 字节对齐
        let mut user_sp = task_inner.res.as_mut().unwrap().user_stack_top();
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();

        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    new_token,
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();

        *argv[args.len()] = 0;
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            *argv[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_refmut(new_token, p as *mut u8) = *c;
                p += 1;
            }
            *translated_refmut(new_token, p as *mut u8) = 0;
        }
        user_sp -= user_sp % core::mem::size_of::<usize>();

        let mut trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            task.kernel_stack.get_top(),
            trap_handler as *const () as usize,
        );
        trap_cx.x[10] = args.len();
        trap_cx.x[11] = argv_base;
        *task_inner.get_trap_cx() = trap_cx;
    }
}
