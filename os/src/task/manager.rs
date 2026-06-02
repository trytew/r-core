use crate::sync::UpIntrFreeCell;
use crate::task::process::ProcessControlBlock;
use crate::task::task::{TaskControlBlock, TaskStatus};
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    // 线程管理器
    pub static ref TASK_MANAGER: UpIntrFreeCell<TaskManager> =
        unsafe { UpIntrFreeCell::new(TaskManager::new()) };

    // 进程map [pid]进程控制块
    pub static ref PID2PCB: UpIntrFreeCell<BTreeMap<usize, Arc<ProcessControlBlock>>> =
        unsafe { UpIntrFreeCell::new(BTreeMap::new()) };
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
    /// 初始化进程管理器
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
    /// 将进程添加到管理器的最后一位
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/5
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    ///
    /// 弹出第一个进程
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/6
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }

    ///
    /// 从待运行队列移除线程
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/21
    pub fn remove(&mut self, task: Arc<TaskControlBlock>) {
        if let Some((id, _)) = self
            .ready_queue
            .iter()
            .enumerate()
            .find(|(_, t)| Arc::as_ptr(t) == Arc::as_ptr(&task))
        {
            self.ready_queue.remove(id);
        }
    }
}

///
/// 添加线程
///
/// @author: tryte
///
/// @date: 2026/3/5
pub fn add_task(task: Arc<TaskControlBlock>) {
    // 将线程控制块加入线程管理器
    TASK_MANAGER.exclusive_access().add(task);
}

///
/// 唤醒进程
///
/// @author: tryte
///
/// @date: 2026/5/19
pub fn wakeup_task(task: Arc<TaskControlBlock>) {
    let mut task_inner = task.inner_exclusive_access();
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    add_task(task);
}

///
/// 移除线程
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn remove_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().remove(task);
}

///
/// 轮询进程
///
/// @author: tryte
///
/// @date: 2026/3/6
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}

///
/// 根据 pid 查找线程
///
/// @author: tryte
///
/// @date: 2026/5/15
pub fn pid2process(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    let map = PID2PCB.exclusive_access();
    map.get(&pid).map(Arc::clone)
}

///
/// 添加线程到线程索引map
///
/// @author: tryte
///
/// @date: 2026/5/19
pub fn insert_into_pid2process(pid: usize, process: Arc<ProcessControlBlock>) {
    PID2PCB.exclusive_access().insert(pid, process);
}

///
/// 移除 pid 索引
///
/// @author: tryte
///
/// @date: 2026/5/15
pub fn remove_from_pid2process(pid: usize) {
    let mut map = PID2PCB.exclusive_access();
    if map.remove(&pid).is_none() {
        panic!("cannot find pid {} in pid2task!", pid)
    }
}
