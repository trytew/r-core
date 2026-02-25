use crate::config::{kernel_stack_position, TRAP_CONTEXT};
use crate::mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::sync::UpSafeCell;
use crate::task::context::TaskContext;
use crate::task::pid::{KernelStack, PidHandle};
use crate::trap::{trap_handler, TrapContext};
use alloc::rc::Weak;
use alloc::sync::Arc;
use core::cell::RefMut;

///
/// 进程状态
///
/// “内核里任务切换时”，内核线程自己的现场
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

pub struct TaskControlBlockInner {
    pub trap_cx_ppn: PhysPageNum, // 进程“陷入”上下文的物理地址
    #[allow(unused)]
    pub base_size: usize,
    pub task_cx: TaskContext,                    // 进程上下文
    pub task_status: TaskStatus,                 // 进程状态
    pub memory_set: MemorySet,                   // 进程内存区域
    pub parent: Option<Weak<TaskControlBlock>>,  // 父进程
    pub children: Option<Arc<TaskControlBlock>>, // 子进程
    pub exit_code: i32,                          // 退出状态值
}

///
/// 进程控制块
///
/// @author: tryte
///
/// @date: 2025/12/18
pub struct TaskControlBlock {
    pub pid: PidHandle,
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
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // 获取进程内存区域集合
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        // 获取进程“陷入”上下文的物理地址
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let pid_handle = pid_alloc();

        // 设置进程状态为待运行
        let task_status = TaskStatus::Ready;

        // 创建内核栈
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );

        // 创建进程控制器
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
        };

        // 创建“陷入”上下文，这里看起来是在直接操作物理地址，但是因为在内核态的情况下（已经使用了内核页表的 MMU 设置），所以这里还是使用
        // 虚拟内存地址访问，因为内核态下页表的虚拟内存地址使用的恒等映射。
        // 这里记录“陷入”上下文的物理地址也是因为这个上下文只在内核态下会用到
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as *const () as usize,
        );

        task_control_block
    }

    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlock> {
        self.inner.inner_exclusive_access()
    }

    ///
    /// 返回进程“陷入”上下文的物理地址
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
    /// 堆虚拟地址空间管理
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/2
    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        let old_break = self.program_brk;
        let new_break = self.program_brk as isize + size as isize;
        if new_break < self.heap_bottom as isize {
            return None;
        }
        let result = if size < 0 {
            // 回收堆空间
            self.memory_set
                .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_break as usize))
        } else {
            // 分配堆空间
            self.memory_set
                .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_break as usize))
        };
        if result {
            self.program_brk = new_break as usize;
            Some(old_break)
        } else {
            None
        }
    }

    ///
    /// fork 新进程
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/5
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        // 获取父进程的 PCB 信息块
        let mut parent_inner;
        todo!()
    }
}
