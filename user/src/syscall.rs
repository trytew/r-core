use crate::signal::SignalAction;
use core::arch::asm;

///
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
const SYSCALL_SIG_ACTION: usize = 134;

/// 屏蔽信号中断号
const SYSCALL_SIG_PROC_MASK: usize = 135;

/// 信号执行返回中断号
const SYSCALL_SIG_RETURN: usize = 139;

/// 获取时间中断号
const SYSCALL_GET_TIME: usize = 169;

/// 获取进程ID中断号
const SYSCALL_GETPID: usize = 172;

/// 创建子进程中断号
const SYSCALL_FORK: usize = 220;

/// 执行新进程中断号
const SYSCALL_EXEC: usize = 221;

/// 等待进程结束中断号
const SYSCALL_WAITPID: usize = 260;

/// 创建线程中断号
const SYSCALL_THREAD_CREATE: usize = 1000;

/// 获取线程ID
const SYSCALL_GET_TID: usize = 1001;

/// 等待线程结束中断号
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

///
/// 系统调用
///
/// @author: tryte
///
/// @date: 2025/11/20
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
        "ecall",
        inlateout("x10") args[0] => ret,
        in("x11") args[1],
        in("x12") args[2],
        in("x17") id,
        )
    }
    ret
}

pub fn sys_dup(fd: usize) -> isize {
    syscall(SYSCALL_DUP, [fd, 0, 0])
}

///
/// 系统打开文件
///
/// @author: tryte
///
/// @date: 2026/4/8
pub fn sys_open(path: &str, flags: u32) -> isize {
    syscall(SYSCALL_OPEN, [path.as_ptr() as usize, flags as usize, 0])
}

///
/// 系统关闭文件
///
/// @author: tryte
///
/// @date: 2026/4/8
pub fn sys_close(fd: usize) -> isize {
    syscall(SYSCALL_CLOSE, [fd, 0, 0])
}

///
/// 创建管道
///
/// @author: tryte
///
/// @date: 2026/4/17
pub fn sys_pipe(pipe: &mut [usize]) -> isize {
    syscall(SYSCALL_PIPE, [pipe.as_mut_ptr() as usize, 0, 0])
}

///
/// 系统读
///
/// @author: tryte
///
/// @date: 2026/3/10
pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall(
        SYSCALL_READ,
        [fd, buffer.as_mut_ptr() as usize, buffer.len()],
    )
}

///
/// 系统写
///
/// @author: tryte
///
/// @date: 2025/11/20
pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

///
/// 系统退出
///
/// @author: tryte
///
/// @date: 2025/11/20
pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0]);
    panic!("sys exit never returns");
}

///
/// 休眠
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn sys_sleep(sleep_ms: usize) -> isize {
    syscall(SYSCALL_SLEEP, [sleep_ms, 0, 0])
}

///
/// 让出时间片
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

///
/// 发送信号
///
/// @author: tryte
///
/// @date: 2026/5/18
pub fn sys_kill(pid: usize, signal: i32) -> isize {
    syscall(SYSCALL_KILL, [pid, signal as usize, 0])
}

///
/// 设置信号响应动作
///
/// @author: tryte
///
/// @date: 2026/5/18
pub fn sys_sigaction(
    signum: i32,
    action: *const SignalAction,
    old_action: *mut SignalAction,
) -> isize {
    // 调用
    syscall(
        SYSCALL_SIG_ACTION,
        [signum as usize, action as usize, old_action as usize],
    )
}

///
/// 屏蔽信号
///
/// @author: tryte
///
/// @date: 2026/5/18
pub fn sys_sig_proc_mask(mask: u32) -> isize {
    syscall(SYSCALL_SIG_PROC_MASK, [mask as usize, 0, 0])
}

///
/// 信号执行返回
///
/// @author: tryte
///
/// @date: 2026/5/18
pub fn sys_sig_return() -> isize {
    syscall(SYSCALL_SIG_RETURN, [0, 0, 0])
}

///
/// 获取时间
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn sys_get_time() -> isize {
    syscall(SYSCALL_GET_TIME, [0, 0, 0])
}

///
/// 获取进程ID
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn sys_getpid() -> isize {
    syscall(SYSCALL_GETPID, [0, 0, 0])
}

///
/// 创建子进程
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn sys_fork() -> isize {
    syscall(SYSCALL_FORK, [0, 0, 0])
}

///
/// 执行新进程
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn sys_exec(path: &str, args: &[*const u8]) -> isize {
    syscall(
        SYSCALL_EXEC,
        [path.as_ptr() as usize, args.as_ptr() as usize, 0],
    )
}

///
/// 等待进程结束
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall(SYSCALL_WAITPID, [pid as usize, exit_code as usize, 0])
}

///
/// 创建线程
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn sys_thread_create(entry: usize, arg: usize) -> isize {
    syscall(SYSCALL_THREAD_CREATE, [entry, arg, 0])
}

///
/// 获取线程ID
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn sys_get_tid() -> isize {
    syscall(SYSCALL_GET_TID, [0; 3])
}

///
/// 等待线程结束
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn sys_wait_tid(tid: usize) -> isize {
    syscall(SYSCALL_WAIT_TID, [tid, 0, 0])
}

///
/// 创建线程锁
///
/// @author: tryte
///
/// @date: 2026/5/28
pub fn sys_mutex_create(blocking: bool) -> isize {
    syscall(SYSCALL_MUTEX_CREATE, [blocking as usize, 0, 0])
}

///
/// 上锁
///
/// @author: tryte
///
/// @date: 2026/5/28
pub fn sys_mutex_lock(id: usize) -> isize {
    syscall(SYSCALL_MUTEX_LOCK, [id, 0, 0])
}

///
/// 解锁
///
/// @author: tryte
///
/// @date: 2026/5/28
pub fn sys_mutex_unlock(id: usize) -> isize {
    syscall(SYSCALL_MUTEX_UNLOCK, [id, 0, 0])
}

///
/// 创建信号量
///
/// @author: tryte
///
/// @date: 2026/5/29
pub fn sys_semaphore_create(res_count: usize) -> isize {
    syscall(SYSCALL_SEMAPHORE_CREATE, [res_count, 0, 0])
}

///
/// 增加信号量
///
/// @author: tryte
///
/// @date: 2026/5/29
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    syscall(SYSCALL_SEMAPHORE_UP, [sem_id, 0, 0])
}

///
/// 减少信号量
///
/// @author: tryte
///
/// @date: 2026/5/29
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    syscall(SYSCALL_SEMAPHORE_DOWN, [sem_id, 0, 0])
}

///
/// 创建条件变量
///
/// @author: tryte
///
/// @date: 2026/5/29
pub fn sys_condvar_create() -> isize {
    syscall(SYSCALL_CONDVAR_CREATE, [0, 0, 0])
}

///
/// 释放条件变量
///
/// @author: tryte
///
/// @date: 2026/5/29
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    syscall(SYSCALL_CONDVAR_SIGNAL, [condvar_id, 0, 0])
}

///
/// 等待条件变量
///
/// @author: tryte
///
/// @date: 2026/5/29
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    syscall(SYSCALL_CONDVAR_WAIT, [condvar_id, mutex_id, 0])
}
