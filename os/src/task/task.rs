use crate::mm::PhysPageNum;
use crate::sync::UpSafeCell;
use crate::task::context::TaskContext;
use crate::task::id::{kernel_stack_alloc, KernelStack, TaskUserRes};
use crate::task::process::ProcessControlBlock;
use crate::trap::TrapContext;
use alloc::sync::{Arc, Weak};
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
    pub res: Option<TaskUserRes>,
    /// 应用“陷入”上下文的物理地址
    pub trap_cx_ppn: PhysPageNum,
    /// 应用“陷入”上下文
    pub task_cx: TaskContext,
    /// 应用状态
    pub task_status: TaskStatus,
    /// 退出状态值
    pub exit_code: Option<i32>,
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
    /// 获取进程状态
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/7
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }
}

///
/// 进程控制块
///
/// @author: tryte
///
/// @date: 2025/12/18
pub struct TaskControlBlock {
    pub process: Weak<ProcessControlBlock>,
    #[allow(unused)]
    pub kernel_stack: KernelStack,
    inner: UpSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
    pub fn new(
        process: Arc<ProcessControlBlock>,
        user_stack_base: usize,
        alloc_user_res: bool,
    ) -> Self {
        let res = TaskUserRes::new(Arc::clone(&process), user_stack_base, alloc_user_res);
        let trap_cx_ppn = res.trap_cx_ppn();
        let kernel_stack = kernel_stack_alloc();
        let kernel_stack_top = kernel_stack.get_top();
        Self {
            process: Arc::downgrade(&process),
            kernel_stack,
            inner: unsafe {
                UpSafeCell::new(TaskControlBlockInner {
                    res: Some(res),
                    trap_cx_ppn,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    exit_code: None,
                })
            },
        }
    }

    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }

    pub fn get_user_token(&self) -> usize {
        let process = self.process.upgrade().unwrap();
        let inner = process.inner_exclusive_access();
        inner.memory_set.token()
    }
}
