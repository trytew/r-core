use crate::println;
use crate::task::exit_current_and_run_next;

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