use crate::config::MAX_APP_NUM;
use crate::sync::UpSafeCell;
use crate::task::context::TaskContext;
use crate::task::switch::__switch;
use crate::task::task::{TaskControlBlock, TaskStatus};

mod context;
mod switch;
mod task;

///
/// 任务管理器内容
///
/// @author: tryte
///
/// @date: 2025/12/18
pub struct TaskManagerInner {
    tasks: [TaskControlBlock; MAX_APP_NUM], // 任务列表（进程列表）
    current_task: usize, // 当前任务编号
}

///
/// 任务管理器
///
/// @author: tryte
///
/// @date: 2025/12/18
pub struct TaskManager {
    num_app: usize, // 任务总数量
    inner: UpSafeCell<TaskManagerInner>, // 获取可变值
}

impl TaskManager {
    ///
    /// 运行第一个任务
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/18
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task_0 = &mut inner.tasks[0];
        task_0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task_0.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!")
    }

    ///
    /// 将当前运行中的任务修改为待运行
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/18
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    ///
    /// 将当前运行中的任务修改为退出
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/18
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    ///
    /// 查找下一个可运行的任务
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/18
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| {
                id % self.num_app
            })
            .find(|id| {
                inner.tasks[*id].task_status == TaskStatus::Ready
            })
    }

    fn run_next_app(&self) {
        if let Some(next) = self.find_next_task() {}
    }
}