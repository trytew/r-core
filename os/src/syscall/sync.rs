use crate::task::{block_current_and_run_next, current_task};
use crate::timer::{add_timer, get_time_ms};

///
/// 休眠系统调用
///
/// @author: tryte
///
/// @date: 2026/5/20
pub fn sys_sleep(ms: usize) -> isize {
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
