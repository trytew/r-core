use crate::fs::{open_file, OpenFlags};
use crate::println;
use crate::sbi::shutdown;
use crate::task::context::TaskContext;
pub use crate::task::manager::add_task;
use crate::task::task::{TaskControlBlock, TaskStatus};
use alloc::sync::Arc;
use lazy_static::*;
pub use processor::*;

mod context;
mod manager;
mod pid;
mod processor;
mod switch;
mod task;

lazy_static! {
    // 实例化初始进程
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        // 读取初始进程内容
        let inode = open_file("initproc", OpenFlags::RDONY).unwrap();
        let v = inode.read_all();
        // 创建初始进程控制块
        TaskControlBlock::new(v.as_slice())
    });
}

///
/// 添加初始应用
///
/// @author: tryte
///
/// @date: 2026/3/5
pub fn add_initproc() {
    add_task(INITPROC.clone());
}

///
/// pid of usertests app in make run TEST=1
///
/// @author: tryte
///
/// @date: 2026/3/6
pub const IDLE_PID: usize = 0;

///
/// 退出当前进程运行下一个进程
///
/// @author: tryte
///
/// @date: 2025/12/22
pub fn exit_current_and_run_next(exit_code: i32) {
    // 获取当前进程的控制块
    let task = take_current_task().unwrap();

    // 当首个进程退出时系统关闭
    let pid = task.getpid();
    if pid == IDLE_PID {
        println!(
            "[kernel] Idle process exit with exit code {} ...",
            exit_code
        );
        if exit_code != 0 {
            shutdown(true)
        } else {
            shutdown(false)
        }
    }

    // 将当前进程设置为僵尸态
    let mut inner = task.inner_exclusive_access();
    inner.task_status = TaskStatus::Zombie;
    inner.exit_code = exit_code;

    {
        // 获取当前进程的子进程，将所有子进程的父进程设置为初始进程，并将子进程变更为初始进程的子进程
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }

    // 清空子进程
    inner.children.clear();
    // 回收当前进程内存
    inner.memory_set.recycle_data_pages();
    // 释放借用，因为 schedule 不会返回，不释放借用会导致生命周期和所有权出问题
    drop(inner);
    drop(task);

    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

///
/// 暂停当前进程运行下一个进程
///
/// @author: tryte
///
/// @date: 2025/12/22
pub fn suspend_current_and_run_next() {
    let task = take_current_task().unwrap();

    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;

    // 设置当前进程为待运行，并添加到待运行队列最后
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    add_task(task);

    // 调度运行下一个进程
    schedule(task_cx_ptr);
}
