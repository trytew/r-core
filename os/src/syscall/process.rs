use crate::println;
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next};
use crate::timer::get_time_ms;

///
/// 退出
///
/// @author: tryte
///
/// @date: 2025/12/10
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}\n", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

///
/// 切换下一个应用
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

///
/// 获取系统时间
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_sbrk(size: i32) -> isize {
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
