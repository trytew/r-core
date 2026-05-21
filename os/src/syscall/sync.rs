use crate::task::{block_current_and_run_next, current_task};
use crate::timer::{add_timer, get_time_ms};

///
/// 休眠系统调用
///
/// @author: tryte
///
/// @date: 2026/5/20
pub fn sys_sleep(ms: usize) -> isize {
    // 计算过期时间
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    // 添加定时器
    add_timer(expire_ms, task);
    // 将进程设置为阻塞，时间未到时不再运行，由定时器唤醒
    block_current_and_run_next();
    0
}
