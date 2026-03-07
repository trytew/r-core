use core::arch::asm;

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

/// 等待进程结束中断号
const SYSCALL_WAITPID: usize = 260;

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
pub fn sys_exit(exit_code: i32) -> isize {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0])
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
pub fn sys_exec(path: &str) -> isize {
    syscall(SYSCALL_EXEC, [path.as_ptr() as usize, 0, 0])
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
