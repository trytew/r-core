use crate::sync::UpSafeCell;
use crate::task::context::TaskContext;
use crate::task::task::TaskControlBlock;
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
    idle_task_cs: TaskContext,
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
            idle_task_cs: TaskContext::zero_init(),
        }
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
