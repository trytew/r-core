use crate::loader::{get_app_data, get_num_app};
use crate::println;
use crate::sbi::shutdown;
use crate::sync::UpSafeCell;
use crate::task::context::TaskContext;
use crate::task::switch::__switch;
use crate::task::task::TaskControlBlock;
use crate::task::task::TaskStatus;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::lazy_static;

mod context;
mod switch;
mod task;

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {

        println!("init TASK_MANAGER");

        // 获取 app 数量
        let num_app = get_num_app();

        // 初始化应用
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }

        TaskManager{
            num_app,
            inner: unsafe {
                UpSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
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
    tasks: Vec<TaskControlBlock>, // 任务列表（应用列表）
    current_task: usize,          // 当前任务编号
}

///
/// 应用管理器
///
/// @author: tryte
///
/// @date: 2025/12/18
pub struct TaskManager {
    num_app: usize,                      // 任务总数量
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
        // 设置第一个应用状态为运行中
        task_0.task_status = TaskStatus::Running;
        // 获取第一个应用的上下文
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
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    ///
    /// 获取当前应用的 MMU 设置
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/31
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    ///
    /// 获取当前应用“陷入”处理函数的物理地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/2
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    ///
    /// 修改程序堆空间大小
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/3
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        // 在增减应用堆空间的过程中全程使用的都是直接堆应用虚拟内存地址的增减，没有涉及任何内核虚拟地址，
        // 并且所有应用内存都设置的内核态（特权级S-mode）下可操作，因此可以在内核空间的状态下对应用的内存进行操作
        inner.tasks[cur].change_program_brk(size)
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

            // 由于 __switch 走完之后就会直接跳转到别的应用执行，因此该函数不会返回，要提前 drop 对象，否则会有生命周期错误的影响
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

///
/// 获取当前应用的 MMU 设置
///
/// @author: tryte
///
/// @date: 2026/2/2
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

///
/// 获取当前应用“陷入”处理函数的物理地址
///
/// @author: tryte
///
/// @date: 2026/2/2
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

///
/// 修改程序堆空间大小
///
/// @author: tryte
///
/// @date: 2026/2/3
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}
