use crate::sync::UpSafeCell;
use crate::task::context::TaskContext;
use crate::task::manager::fetch_task;
use crate::task::switch::__switch;
use crate::task::task::{TaskControlBlock, TaskStatus};
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PROCESSOR: UpSafeCell<Processor> = unsafe { UpSafeCell::new(Processor::new()) };
}

///
/// 进程处理器
///
/// @author: tryte
///
/// @date: 2026/2/5
pub struct Processor {
    current: Option<Arc<TaskControlBlock>>,
    idle_task_cx: TaskContext,
}

impl Processor {
    ///
    /// 创建进程处理器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/5
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }

    ///
    /// 获取闲置进程的上下文
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/6
    pub fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }

    ///
    /// 获取正在运行的进程控制块
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/5
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }

    ///
    /// 获取当前进程
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/6
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
}

///
/// 运行应用
///
/// @author: tryte
///
/// @date: 2026/3/6
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            // 获取空闲进程的应用上下文地址
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // 获取即将运行的进程的应用上下文地址
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            // 修改进程状态
            task_inner.task_status = TaskStatus::Running;
            // 释放借用
            drop(task_inner);
            // 将当前进程控制块设置为即将运行的进程控制块
            processor.current = Some(task);
            // 释放借用
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        }
    }
}

///
/// 获取正在运行的进程控制块
///
/// @author: tryte
///
/// @date: 2026/2/5
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

///
/// 获取当前应用的虚拟地址页表设置
///
/// @author: tryte
///
/// @date: 2026/3/6
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.inner_exclusive_access().get_user_token();
    token
}

///
/// 获取当前应用的“陷入”上下文
///
/// @author: tryte
///
/// @date: 2026/3/6
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

///
/// 获取当前进程
///
/// @author: tryte
///
/// @date: 2026/3/6
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

///
/// 调度进程
///
/// @author: tryte
///
/// @date: 2026/3/6
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processer = PROCESSOR.exclusive_access();
    // 获取 idle 进程
    let idle_task_cx_ptr = processer.get_idle_task_cx_ptr();
    drop(processer);

    unsafe {
        // 切换运行进程，当切换回 idle 进程后会继续 task::run_tasks() 中 loop 的下一个循环，继续从待运行进程队列中获取进程运行
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
