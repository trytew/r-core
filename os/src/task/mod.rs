use crate::fs::{open_file, OpenFlags};
use crate::println;
use crate::sbi::shutdown;
pub use crate::task::context::TaskContext;
pub(crate) use crate::task::task::{TaskControlBlock, TaskStatus};
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::*;

use crate::task::id::TaskUserResource;
pub use crate::task::manager::*;
use crate::task::process::ProcessControlBlock;
use crate::timer::remove_timer;
pub use processor::*;
pub use signal::*;

mod context;
mod id;
mod manager;
mod process;
mod processor;
mod signal;
mod switch;
mod task;

lazy_static! {
    // 实例化初始进程
    pub static ref INITPROC: Arc<ProcessControlBlock> = {
        // 读取初始进程内容
        let inode = open_file("initproc", OpenFlags::RDONY).unwrap();
        let v = inode.read_all();
        // 创建初始进程控制块
        ProcessControlBlock::new(v.as_slice())
    };
}
///
/// pid of usertests app in make run TEST=1
///
/// @author: tryte
///
/// @date: 2026/3/6
pub const IDLE_PID: usize = 0;

///
/// 添加初始应用
///
/// @author: tryte
///
/// @date: 2026/3/5
pub fn add_initproc() {
    let _initproc = INITPROC.clone();
}

///
/// 暂停当前线程运行下一个线程
///
/// @author: tryte
///
/// @date: 2025/12/22
pub fn suspend_current_and_run_next() {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // 设置当前线程为待运行，并添加到待运行队列最后
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    add_task(task);

    // 调度运行下一个线程
    schedule(task_cx_ptr);
}

///
/// 阻塞当前线程
///
/// @author: tryte
///
/// @date: 2026/6/2
pub fn block_current_task() -> *mut TaskContext {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.task_status = TaskStatus::Blocked;
    &mut task_inner.task_cx as *mut TaskContext
}

///
/// 阻塞当前线程运行下一个线程
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn block_current_and_run_next() {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    let task_ctx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // 设置当前线程为阻塞态
    task_inner.task_status = TaskStatus::Blocked;
    drop(task_inner);
    schedule(task_ctx_ptr);
}

///
/// 退出当前进程运行下一个进程
///
/// @author: tryte
///
/// @date: 2025/12/22
pub fn exit_current_and_run_next(exit_code: i32) {
    // 获取当前进程的控制块
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let process = task.process.upgrade().unwrap();
    let tid = task_inner.res.as_ref().unwrap().tid;

    // 线程标识退出
    task_inner.exit_code = Some(exit_code);
    // 释放用户态资源
    task_inner.res = None;
    // 释放借用，因为 schedule 不会返回，不释放借用会导致生命周期和所有权出问题
    drop(task_inner);
    drop(task);

    if tid == 0 {
        // 主线程退出
        let pid = process.getpid();
        // 当初始进程退出时系统关闭
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
        // 移除进程索引 map
        remove_from_pid2process(pid);

        // 将当前进程设置为僵尸态
        let mut process_inner = process.inner_exclusive_access();
        process_inner.is_zombie = true;
        process_inner.exit_code = exit_code;

        {
            // 获取当前进程的子进程，将所有子进程的父进程设置为初始进程，并将子进程变更为初始进程的子进程
            let mut initproc_inner = INITPROC.inner_exclusive_access();
            for child in process_inner.children.iter() {
                child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
                initproc_inner.children.push(child.clone());
            }
        }

        // 移除所有线程的用户资源
        // 它必须在锁定整个 memory_set 之前完成否则它们将被释放两次
        let mut recycle_res = Vec::<TaskUserResource>::new();
        for task in process_inner.tasks.iter().filter(|t| t.is_some()) {
            let task = task.as_ref().unwrap();
            // 如果线程管理器中有其他线程准备就绪或正在等待计时器过期，需要移除。
            // 不需要考虑互斥体/信号量，因为它们被限制在单个进程中。因此，被阻止的任务是当PCB被解除分配时移除。
            remove_inactive_task(Arc::clone(&task));
            // 取出用户态资源
            let mut task_inner = task.inner_exclusive_access();
            if let Some(res) = task_inner.res.take() {
                recycle_res.push(res);
            }
        }

        drop(process_inner);
        // 释放所有子线程的用户态资源
        recycle_res.clear();

        // 清理进程资源
        let mut process_inner = process.inner_exclusive_access();
        process_inner.children.clear();
        process_inner.memory_set.recycle_data_pages();
        process_inner.fd_table.clear();

        // 弹出所有空线程
        while process_inner.tasks.len() > 1 {
            process_inner.tasks.pop();
        }
    }

    drop(process);
    // 构造无用线程切换到空闲进程
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

///
/// 检查当前进程是否有错误信号
///
/// @author: tryte
///
/// @date: 2026/5/15
pub fn check_signals_of_current() -> Option<(i32, &'static str)> {
    // 获取当前进程控制块
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    // 检查有没有收到出错信号
    process_inner.signals.check_error()
}

pub fn current_add_signal(signal: SignalFlags) {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.signals |= signal;
}

///
/// 回收线程
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn remove_inactive_task(task: Arc<TaskControlBlock>) {
    // 从待运行队列移除
    remove_task(Arc::clone(&task));
    // 从计时器中移除
    remove_timer(Arc::clone(&task));
}
