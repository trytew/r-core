use crate::loader::get_app_data_by_name;
use crate::mm::{translated_refmut, translated_str};
use crate::task::{
    add_task, current_task, current_user_token, exit_current_and_run_next,
    suspend_current_and_run_next,
};
use crate::timer::get_time_ms;
use alloc::sync::Arc;

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

///
/// 获取进程id
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

///
/// 创建子进程
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    // TODO
    let new_pid = new_task.getpid();
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    trap_cx.x[10] = 0;
    add_task(new_task);
    new_pid as isize
}

///
/// 执行新程序
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

///
/// 获取进程状态
///
/// 进程ID不等于-1或者当前进程查找不到该子进程返回 -1
///
/// 如果当前子进程仍处于运行态则返回 -2
///
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();

    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
    }

    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // 当前子进程为僵尸态且查找的进程id为 -1 或和传入的进程id一致
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
    });

    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        let exit_code = child.inner_exclusive_access().exit_code;
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
}
