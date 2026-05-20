use crate::fs::{open_file, OpenFlags};
use crate::mm::{translated_ref, translated_refmut, translated_str};
use crate::task::{
    current_process, current_task, current_user_token, exit_current_and_run_next, pid2process,
    suspend_current_and_run_next, SignalFlags,
};
use crate::timer::get_time_ms;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

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
    current_task().unwrap().process.upgrade().unwrap().getpid() as isize
}

///
/// 创建子进程
///
/// ```
/// fork 的调用在用户态的表现上是返回两次，但实际上只有父进程 fork 的时候会返回一次。子进程则是因为在 fork 的过程中复制了“陷入”上下文，因此子进程中的 spec 对应的是父进程中 fork 调用后的下一个指令地址，又因为现在使用的虚拟地址，而父子进程内容一样，所以当子进程被调度时是从 fork() 之后运行的。先将子进程作为待运行进程放入调度列表中等待，当子进程被调度时会通过 __switch -> __restore 从内核态切换到用户态直接从 spec 执行的指令地址执行，这个时候根据 RISC-V 的 ABI 调用约定，第一返回值寄存器是 x10，而在 fork 的过程中设置了子进程 x10 的值为 0，因此才造成了用户态返回两次的效果
/// ```
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn sys_fork() -> isize {
    // 获取当前进程控制块
    let current_process = current_process();
    let new_process = current_process.fork();
    // 获取新进程的进程ID
    let new_pid = new_process.getpid();
    let new_process_inner = new_process.inner_exclusive_access();
    let task = new_process_inner.tasks[0].as_ref().unwrap();
    // 获取新进程的“陷入”上下文
    let trap_cx = task.inner_exclusive_access().get_trap_cx();
    // 子进程的 fork 返回 0
    trap_cx.x[10] = 0;
    new_pid as isize
}

///
/// 执行新程序
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    let mut args_vec: Vec<String> = Vec::new();
    loop {
        let arg_str_ptr = *translated_ref(token, args);
        if arg_str_ptr == 0 {
            break;
        }
        args_vec.push(translated_str(token, arg_str_ptr as *const u8));
        unsafe {
            args = args.add(1);
        }
    }
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONY) {
        let all_data = app_inode.read_all();
        let process = current_process();
        let argc = args_vec.len();
        process.exec(all_data.as_slice(), args_vec);
        argc as isize
    } else {
        -1
    }
}

///
/// 获取进程状态
///
/// 若 pid = -1 则代表任意一个子进程
///
/// 当前进程查找不到该子进程返回 -1
///
/// 如果当前子进程仍处于运行态则返回 -2
///
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_process();

    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
    }

    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // 当前子进程为僵尸态且查找任意子进程 或和传入的进程id一致
        p.inner_exclusive_access().is_zombie && (pid == -1 || pid as usize == p.getpid())
    });

    // 回收子进程资源
    if let Some((idx, _)) = pair {
        // KernelStack 的 drop 函数触发点
        let child = inner.children.remove(idx);
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // 返回子进程的退出码
        let exit_code = child.inner_exclusive_access().exit_code;
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        // 返回回收的子进程id
        found_pid as isize
    } else {
        -2
    }
}

///
/// 发送信号到进程
///
/// @author: tryte
///
/// @date: 2026/5/15
pub fn sys_kill(pid: usize, signum: i32) -> isize {
    if let Some(process) = pid2process(pid) {
        if let Some(flag) = SignalFlags::from_bits(1 << signum) {
            let mut task_ref = process.inner_exclusive_access();
            if task_ref.signals.contains(flag) {
                return -1;
            }
            task_ref.signals.insert(flag);
            0
        } else {
            -1
        }
    } else {
        -1
    }
}
