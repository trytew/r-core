use crate::config::TRAP_CONTEXT;
use crate::mm::{MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::sync::UpSafeCell;
use crate::task::context::TaskContext;
use crate::task::pid::{pid_alloc, KernelStack, PidHandle};
use crate::trap::{trap_handler, TrapContext};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::cell::RefMut;

///
/// 应用状态
///
/// “内核里进程切换时”，内核线程自己的现场
/// 保存当前内核执行流的状态，方便以后从这里继续跑
///
/// @author: tryte
///
/// @date: 2025/12/18
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,   // 待运行
    Running, // 运行中
    Zombie,  // 僵尸态
}

///
/// 进程控制块
///
/// @author: tryte
///
/// @date: 2026/3/6
pub struct TaskControlBlockInner {
    pub trap_cx_ppn: PhysPageNum, // 应用“陷入”上下文的物理地址
    #[allow(unused)]
    pub base_size: usize,
    pub task_cx: TaskContext,                   // 应用“陷入”上下文
    pub task_status: TaskStatus,                // 应用状态
    pub memory_set: MemorySet,                  // 应用内存区域
    pub parent: Option<Weak<TaskControlBlock>>, // 父进程
    pub children: Vec<Arc<TaskControlBlock>>,   // 子进程
    pub exit_code: i32,                         // 退出状态值
}

impl TaskControlBlockInner {
    ///
    /// 返回应用“陷入”上下文的物理地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/30
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    ///
    /// 获取用户空间的 MMU 设置
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/31
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    ///
    /// 获取进程状态
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/7
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }

    ///
    /// 判断进程是否存于僵尸态
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/7
    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
}

///
/// 进程控制块
///
/// @author: tryte
///
/// @date: 2025/12/18
pub struct TaskControlBlock {
    pub pid: PidHandle,
    #[allow(unused)]
    pub kernel_stack: KernelStack,
    inner: UpSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
    ///
    /// 创建进程控制器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/30
    pub fn new(elf_data: &[u8]) -> Self {
        // 获取进程内存区域集合
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        // 获取应用“陷入”上下文的物理地址，因为在“陷入”处理的时候处于内核态，因此需要记录真实的物理地址才能找到对应应用的“陷入”上下文
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // 分配进程id
        let pid_handle = pid_alloc();
        // 为进程创建内核栈
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();

        // 创建进程控制器，这里记录“陷入”上下文的物理地址也是因为这个上下文只在内核态下会用到
        let task_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UpSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                })
            },
        };

        // 创建“陷入”上下文，这里看起来是在直接操作物理地址，但是因为在内核态的情况下（已经使用了内核页表的 MMU 设置），所以这里还是使用
        // 虚拟内存地址访问，因为内核态下页表的虚拟内存地址使用的恒等映射。
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();

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
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as *const () as usize,
        );

        task_control_block
    }

    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
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
        let mut parent_inner = self.inner_exclusive_access();
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UpSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: parent_inner.base_size,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                })
            },
        });

        parent_inner.children.push(task_control_block.clone());

        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;

        task_control_block
    }

    ///
    /// 执行新程序
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/7
    pub fn exec(&self, elf_data: &[u8]) {
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let mut inner = self.inner_exclusive_access();
        inner.memory_set = memory_set;
        inner.trap_cx_ppn = trap_cx_ppn;
        inner.base_size = user_sp;
        let trap_cx = inner.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
    }
}
