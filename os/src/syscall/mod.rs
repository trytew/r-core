use crate::syscall::fs::{sys_close, sys_dup, sys_open, sys_pipe, sys_read, sys_write};
use crate::syscall::process::{
    sys_exec, sys_exit, sys_fork, sys_get_time, sys_getpid, sys_kill, sys_sig_proc_mask,
    sys_sig_return, sys_sigaction, sys_waitpid, sys_yield,
};
use crate::task::SignalAction;

mod fs;
mod process;

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

/// 时间中断号
const SYSCALL_YIELD: usize = 124;

/// 发送信号中断号
const SYSCALL_KILL: usize = 129;

/// 设置信号执行函数中断号
const SYSCALL_SIGACTION: usize = 134;

/// 设置屏蔽信号中断号
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

/// 等待进程组中断号
const SYSCALL_WAITPID: usize = 260;

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
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_KILL => sys_kill(args[0], args[1] as i32),
        SYSCALL_SIGACTION => sys_sigaction(
            args[0] as i32,
            args[1] as *const SignalAction,
            args[2] as *mut SignalAction,
        ),
        SYSCALL_SIG_PROC_MASK => sys_sig_proc_mask(args[0] as u32),
        SYSCALL_SIG_RETURN => sys_sig_return(),
        SYSCALL_GET_TIME => sys_get_time(),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8, args[1] as *const usize),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
