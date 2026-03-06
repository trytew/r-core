use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};
use crate::timer::get_time_ms;

///
/// 退出
///
/// @author: tryte
///
/// @date: 2025/12/10
pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
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
