use crate::syscall::fs::{sys_close, sys_dup, sys_open, sys_pipe, sys_read, sys_write};
use crate::syscall::gui::{sys_framebuffer, sys_framebuffer_flush};
use crate::syscall::input::{sys_event_get, sys_key_pressed};
use crate::syscall::process::{
    sys_exec, sys_exit, sys_fork, sys_get_time, sys_getpid, sys_kill, sys_waitpid, sys_yield,
};
use crate::syscall::sync::{
    sys_condvar_create, sys_condvar_signal, sys_condvar_wait, sys_mutex_create, sys_mutex_lock,
    sys_mutex_unlock, sys_semaphore_create, sys_semaphore_down, sys_semaphore_up, sys_sleep,
};
use crate::syscall::thread::{sys_get_tid, sys_thread_create, sys_wait_tid};

mod fs;
mod gui;
mod input;
mod process;
mod sync;
pub mod thread;

/// 复制文件描述符中断号
const SYSCALL_DUP: usize = 24;

/// 打开中断号
const SYSCALL_OPEN: usize = 56;

/// 关闭中断号
const SYSCALL_CLOSE: usize = 57;

/// 创建管道
const SYSCALL_PIPE: usize = 59;

/// 读中断号
const SYSCALL_READ: usize = 63;

/// 写中断号
const SYSCALL_WRITE: usize = 64;

/// 退出中断号
const SYSCALL_EXIT: usize = 93;

/// 休眠中断号
const SYSCALL_SLEEP: usize = 101;

/// 时间中断号
const SYSCALL_YIELD: usize = 124;

/// 发送信号中断号
const SYSCALL_KILL: usize = 129;

/// 设置信号执行函数中断号
#[allow(unused)]
const SYSCALL_SIGACTION: usize = 134;

/// 设置屏蔽信号中断号
#[allow(unused)]
const SYSCALL_SIG_PROC_MASK: usize = 135;

/// 信号执行返回中断号
#[allow(unused)]
const SYSCALL_SIG_RETURN: usize = 139;

/// 获取时间中断号
const SYSCALL_GET_TIME: usize = 169;

/// 获取进程ID中断号
const SYSCALL_GETPID: usize = 172;

/// 创建子进程中断号
const SYSCALL_FORK: usize = 220;

/// 执行新进程中断号
const SYSCALL_EXEC: usize = 221;

/// 等待进程组退出中断号
const SYSCALL_WAITPID: usize = 260;

/// 创建线程中断号
const SYSCALL_THREAD_CREATE: usize = 1000;

/// 获取线程id中断号
const SYSCALL_GET_TID: usize = 1001;

/// 等待线程退出中断号
const SYSCALL_WAIT_TID: usize = 1002;

/// 创建线程锁中断号
const SYSCALL_MUTEX_CREATE: usize = 1010;

/// 上锁中断号
const SYSCALL_MUTEX_LOCK: usize = 1011;

/// 解锁中断号
const SYSCALL_MUTEX_UNLOCK: usize = 1012;

/// 创建信号量中断号
const SYSCALL_SEMAPHORE_CREATE: usize = 1020;

/// 增加信号量中断号
const SYSCALL_SEMAPHORE_UP: usize = 1021;

/// 减少信号量中断号
const SYSCALL_SEMAPHORE_DOWN: usize = 1022;

/// 创建条件变量中断号
const SYSCALL_CONDVAR_CREATE: usize = 1030;

/// 释放条件变量中断号
const SYSCALL_CONDVAR_SIGNAL: usize = 1031;

/// 等待条件变量中断号
const SYSCALL_CONDVAR_WAIT: usize = 1032;

/// 分配缓存页中断号
const SYSCALL_FRAMEBUFFER: usize = 2000;

/// 将数据刷入缓存页中断号
const SYSCALL_FRAMEBUFFER_FLUSH: usize = 2001;

/// 获取事件中断
const SYSCALL_EVENT_GET: usize = 3000;

/// 键盘输入中断
const SYSCALL_KEY_PRESSED: usize = 3001;

///
/// 系统调用
///
/// @author: tryte
///
/// @date: 2025/12/10
pub fn sys_call(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_DUP => sys_dup(args[0]),
        SYSCALL_OPEN => sys_open(args[0] as *const u8, args[1] as u32),
        SYSCALL_CLOSE => sys_close(args[0]),
        SYSCALL_PIPE => sys_pipe(args[0] as *mut usize),
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_SLEEP => sys_sleep(args[0]),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_KILL => sys_kill(args[0], args[1] as u32),
        SYSCALL_GET_TIME => sys_get_time(),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8, args[1] as *const usize),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_THREAD_CREATE => sys_thread_create(args[0], args[1]),
        SYSCALL_GET_TID => sys_get_tid(),
        SYSCALL_WAIT_TID => sys_wait_tid(args[0]) as isize,
        SYSCALL_MUTEX_CREATE => sys_mutex_create(args[0] == 1),
        SYSCALL_MUTEX_LOCK => sys_mutex_lock(args[0]),
        SYSCALL_MUTEX_UNLOCK => sys_mutex_unlock(args[0]),
        SYSCALL_SEMAPHORE_CREATE => sys_semaphore_create(args[0]),
        SYSCALL_SEMAPHORE_UP => sys_semaphore_up(args[0]),
        SYSCALL_SEMAPHORE_DOWN => sys_semaphore_down(args[0]),
        SYSCALL_CONDVAR_CREATE => sys_condvar_create(),
        SYSCALL_CONDVAR_SIGNAL => sys_condvar_signal(args[0]),
        SYSCALL_CONDVAR_WAIT => sys_condvar_wait(args[0], args[1]),
        SYSCALL_FRAMEBUFFER => sys_framebuffer(),
        SYSCALL_FRAMEBUFFER_FLUSH => sys_framebuffer_flush(),
        SYSCALL_EVENT_GET => sys_event_get(),
        SYSCALL_KEY_PRESSED => sys_key_pressed(),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
