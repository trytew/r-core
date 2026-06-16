use crate::mm::kernel_token;
use crate::task::TaskControlBlock;
use crate::task::{add_task, current_task};
use crate::trap::{trap_handler, TrapContext};
use alloc::sync::Arc;

///
/// 创建线程
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn sys_thread_create(entry: usize, arg: usize) -> isize {
    let task = current_task().unwrap();
    let process = task.process.upgrade().unwrap();

    // 创建线程
    let new_task = Arc::new(TaskControlBlock::new(
        Arc::clone(&process),
        task.inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .user_stack_base,
        true,
    ));

    // 加入到待执行队列
    add_task(Arc::clone(&new_task));

    // 获取新线程ID
    let new_task_inner = new_task.inner_exclusive_access();
    let new_task_res = new_task_inner.res.as_ref().unwrap();
    let new_task_tid = new_task_res.tid;

    // 将线程加入到进程
    let mut process_inner = process.inner_exclusive_access();
    let tasks = &mut process_inner.tasks;
    // 线程ID分配器是每个进程独立的，因此由线程ID作为线程组下标
    while tasks.len() < new_task_tid + 1 {
        tasks.push(None);
    }
    tasks[new_task_tid] = Some(Arc::clone(&new_task));

    // 设置新线程的“陷入”上下文内容
    let new_task_trap_cx = new_task_inner.get_trap_cx();
    *new_task_trap_cx = TrapContext::app_init_context(
        entry,
        new_task_res.user_stack_top(),
        kernel_token(),
        new_task.kernel_stack.get_top(),
        trap_handler as *const () as usize,
    );
    (*new_task_trap_cx).x[10] = arg;
    new_task_tid as isize
}

///
/// 获取当前线程ID
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn sys_get_tid() -> isize {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid as isize
}

///
/// 等待线程结束
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn sys_wait_tid(tid: usize) -> i32 {
    // 获取线程和进程的信息
    let task = current_task().unwrap();
    let process = task.process.upgrade().unwrap();
    let task_inner = task.inner_exclusive_access();
    let mut process_inner = process.inner_exclusive_access();

    // 无法等待自身结束
    if task_inner.res.as_ref().unwrap().tid == tid {
        return -1;
    }

    // 查看等待的线程是否已结束
    let mut exit_code: Option<i32> = None;
    let waited_task = process_inner.tasks[tid].as_ref();
    if let Some(waited_task) = waited_task {
        if let Some(waited_exit_code) = waited_task.inner_exclusive_access().exit_code {
            exit_code = Some(waited_exit_code);
        }
    } else {
        return -1;
    }

    // 释放已结束的线程，并返回结束码
    if let Some(exit_code) = exit_code {
        process_inner.tasks[tid] = None;
        exit_code
    } else {
        -2
    }
}
