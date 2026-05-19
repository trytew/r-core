use crate::fs::{open_file, OpenFlags};
use crate::println;
use crate::sbi::shutdown;
use crate::task::context::TaskContext;
use crate::task::manager::remove_from_pid2task;
pub(crate) use crate::task::task::{TaskControlBlock, TaskStatus};
use alloc::sync::Arc;
use lazy_static::*;

pub use crate::task::manager::*;
pub use action::*;
pub use processor::*;
pub use signal::*;
pub use task::*;

mod action;
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

    // 移除进程索引 map
    remove_from_pid2task(task.getpid());

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

///
/// 检查当前进程是否有错误信号
///
/// @author: tryte
///
/// @date: 2026/5/15
pub fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    // 获取当前进程控制块
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();
    // 检查有没有收到出错信号
    task_inner.signals.check_error()
}

pub fn current_add_signal(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.signals |= signal;
}

///
/// 处理内核发送的信号
///
/// @author: tryte
///
/// @date: 2026/5/15
fn call_kernel_signal_handler(signal: SignalFlags) {
    // 获取当前进程控制块
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    match signal {
        SignalFlags::SIGSTOP => {
            // 暂停进程信号
            task_inner.frozen = true;
            // 异或进程信号，消耗信号，代表信号已执行
            task_inner.signals ^= SignalFlags::SIGSTOP;
        }
        SignalFlags::SIGCONT => {
            // 恢复暂停的进程
            if task_inner.signals.contains(SignalFlags::SIGCONT) {
                // 异或进程信号，消耗信号，代表信号已执行
                task_inner.signals ^= SignalFlags::SIGCONT;
                task_inner.frozen = false;
            }
        }
        _ => {
            // 终止进程
            task_inner.killed = true;
        }
    }
}

///
/// 执行用户态发送的信号
///
/// @author: tryte
///
/// @date: 2026/5/15
fn call_user_signal_handler(sig: usize, signal: SignalFlags) {
    // 获取进程控制块
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    // 查找进程设置的处理函数
    let handler = task_inner.signal_actions.table[sig].handler;
    if handler != 0 {
        // 设置信号在处理中
        task_inner.handling_sig = sig as isize;

        // 异或进程信号，消耗信号，代表信号已执行
        task_inner.signals ^= signal;

        // 记录当前进程的“陷入”上下文，用于信号执行后返回进程当前时刻上下文
        let trap_ctx = task_inner.get_trap_cx();
        task_inner.trap_ctx_backup = Some(*trap_ctx);

        // 将进程的下一条执行语句设置为信号执行函数地址
        trap_ctx.sepc = handler;

        // 返回执行的信号
        trap_ctx.x[10] = sig;
    } else {
        println!("[K] task/call_user_signal_handler: default action: ignore it or kill process");
    }
}

///
/// 检查处理等待的信号
///
/// @author: tryte
///
/// @date: 2026/5/15
fn check_pending_signals() {
    // 检查所有信号
    for sig in 0..(MAX_SIG + 1) {
        // 获取进程控制块
        let task = current_task().unwrap();
        let task_inner = task.inner_exclusive_access();
        // 筛选需要检查的信号
        let signal = SignalFlags::from_bits(1 << sig).unwrap();
        // 查看进程是否收到对应的信号且不在屏蔽的信号内
        if task_inner.signals.contains(signal) && (!task_inner.signal_mask.contains(signal)) {
            // 查看信号是否能处理
            let mut masked = true;
            let handling_sig = task_inner.handling_sig;
            // 当正在处理的信号等于 -1 代表没有在处理信号
            if handling_sig == -1 {
                masked = false;
            } else {
                // 若有信号正在执行，检查当前信号是否被临时屏蔽
                let handling_sig = handling_sig as usize;
                if !task_inner.signal_actions.table[handling_sig]
                    .mask
                    .contains(signal)
                {
                    masked = false;
                }
            }

            if !masked {
                drop(task_inner);
                drop(task);
                if signal == SignalFlags::SIGKILL
                    || signal == SignalFlags::SIGSTOP
                    || signal == SignalFlags::SIGCONT
                    || signal == SignalFlags::SIGDEF
                {
                    // 处理内核信号
                    call_kernel_signal_handler(signal);
                } else {
                    // 处理用户信号
                    call_user_signal_handler(sig, signal);
                    return;
                }
            }
        }
    }
}

///
/// 处理信号
///
/// @author: tryte
///
/// @date: 2026/5/15
pub fn handle_signals() {
    loop {
        // 检查并执行等待处理的信号
        check_pending_signals();
        // 获取进程状态
        let (frozen, killed) = {
            let task = current_task().unwrap();
            let task_inner = task.inner_exclusive_access();
            (task_inner.frozen, task_inner.killed)
        };
        // 如果当前进程处于非冻结非结束状态就返回执行
        if !frozen || killed {
            break;
        }
        suspend_current_and_run_next()
    }
}
