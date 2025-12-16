use crate::batch::run_next_app;
use crate::println;

///
/// 退出
///
/// @author: tryte
///
/// @date: 2025/12/10
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    run_next_app();
}