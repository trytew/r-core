use crate::syscall::fs::sys_write;
use crate::syscall::process::{
    sys_exec, sys_exit, sys_fork, sys_get_time, sys_getpid, sys_waitpid, sys_yield,
};

mod fs;
mod process;

/// 写入中断号
const SYSCALL_WRITE: usize = 64;

/// 退出中断号
const SYSCALL_EXIT: usize = 93;

/// 时间中断号
const SYSCALL_YIELD: usize = 124;

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
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
