use core::arch::asm;

/// 写入中断号
const SYSCALL_WRITE: usize = 64;

/// 退出中断号
const SYSCALL_EXIT: usize = 93;

/// 时间中断号
const SYSCALL_YIELD: usize = 124;

/// 获取时间中断号
const SYSCALL_GET_TIME: usize = 169;

/// 调整堆空间中断号
const SYSCALL_SBRK: usize = 214;

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
/// 调整堆空间
///
/// @author: tryte
///
/// @date: 2026/2/3
pub fn sys_sbrk(size: i32) -> isize {
    syscall(SYSCALL_GET_TIME, [size as usize, 0, 0])
}
