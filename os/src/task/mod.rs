use crate::config::MAX_APP_NUM;
use crate::loader::get_num_app;
use crate::loader::init_app_cx;
use crate::println;
use crate::sbi::shutdown;
use crate::sync::UpSafeCell;
use crate::task::context::TaskContext;
use crate::task::switch::__switch;
use crate::task::task::TaskControlBlock;
use crate::task::task::TaskStatus;
use lazy_static::lazy_static;

mod context;
mod switch;
mod task;

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {

        // 获取 app 数量
        let num_app = get_num_app();

        // 实例化任务上下文
        let mut tasks = [TaskControlBlock{
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];

        // 初始化任务
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }

        TaskManager{
            num_app,
            inner: unsafe {
                UpSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task:0,
                })
            }
        }
    };
}

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
        let mut unused = TaskContext::zero_init();
        unsafe {
            __switch(&mut unused as *mut TaskContext, next_task_cx_ptr);
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
        ((current + 1)..(current + self.num_app + 1))
            .map(|id| {
                id % self.num_app
            })
            .find(|id| {
                inner.tasks[*id].task_status == TaskStatus::Ready
            })
    }

    ///
    /// 运行下一个任务
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/22
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            println!("All applications completed!");
            shutdown(false);
        }
    }
}

///
/// 运行第一个任务
///
/// @author: tryte
///
/// @date: 2025/12/22
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

///
/// 运行下一个任务
///
/// @author: tryte
///
/// @date: 2025/12/22
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

///
/// 标记任务暂停
///
/// @author: tryte
///
/// @date: 2025/12/22
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

///
/// 标记任务退出
///
/// @author: tryte
///
/// @date: 2025/12/22
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

///
/// 暂停当前任务运行下一个任务
///
/// @author: tryte
///
/// @date: 2025/12/22
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

///
/// 退出当前任务运行下一个任务
///
/// @author: tryte
///
/// @date: 2025/12/22
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}