#![no_std]
#![feature(linkage)]

pub mod console;
mod lang_items;
mod syscall;

use syscall::*;

// 使用 Rust 的宏将 _start 这段代码编译后的汇编代码中放在一个名为 .text.entry 的代码段中，
// 在 linker.ld 脚本中指定了 .text.entry 在最开始的位置
// 方便我们在后续链接的时候调整它的位置使得它能够作为用户库的入口
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    exit(main());
    panic!("unreachable after sys_exit!")
}

// 使用 Rust 的宏将其函数符号 main 标志为弱链接。
// 这样在最后链接的时候，虽然在 lib.rs 和 bin 目录下的某个应用程序都有 main 符号，但由于 lib.rs 中的 main 符号是弱链接，
// 链接器会使用 bin 目录下的应用主逻辑作为 main 。这里我们主要是进行某种程度上的保护，如果在 bin 目录下找不到任何 main ，
// 那么编译也能够通过，但会在运行时报错
#[linkage = "weak"]
#[unsafe(no_mangle)]
fn main() -> i32 {
    panic!("Cannot found main");
}

///
/// 写入
///
/// @author: tryte
///
/// @date: 2025/11/20
pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

///
/// 退出
///
/// @author: tryte
///
/// @date: 2025/11/20
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}

///
/// 让出时间片
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn yield_() -> isize {
    sys_yield()
}

///
/// 获取时间
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn get_time() -> isize {
    sys_get_time()
}

///
/// 获取进程ID
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn getpid() -> isize {
    sys_getpid()
}

///
/// 创建子进程
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn fork() -> isize {
    sys_fork()
}

///
/// 执行新进程
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn exec(path: &str) -> isize {
    sys_exec(path)
}

///
/// 等待所有进程退出
///
/// @author: tryte
///
/// @date: 2026/3/7
pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            exit_pid => return exit_pid,
        }
    }
}
