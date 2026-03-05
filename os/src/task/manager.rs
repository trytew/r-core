use crate::sync::UpSafeCell;
use crate::task::task::TaskControlBlock;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref TASK_MANAGER: UpSafeCell<TaskManager> =
        unsafe { UpSafeCell::new(TaskManager::new()) };
}

///
/// 应用管理器
///
/// @author: tryte
///
/// @date: 2025/12/18
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl TaskManager {
    ///
    /// 初始化任务管理器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/5
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }

    ///
    /// 将任务添加到管理器的最后一位
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/5
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
}

///
/// 添加任务
///
/// @author: tryte
///
/// @date: 2026/3/5
pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}
