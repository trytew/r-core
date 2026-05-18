// 不加载 std 公共库
#![no_std]
// 这个程序不要生成默认的 main 入口函数
// 正常 Rust 程序，Rust 会自动生成程序入口 _start，在 _start 里调用你的 main
// 因为我们在 lib.rs 中已经定义了 _start 入口并加载了 main 执行，因此不能让 rust 生成默认的入口函数
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, wait, yield_};

///
/// init进程
///
/// @author: tryte
///
/// @date: 2026/3/10
// #[unsafe(no_mangle)]的作用是禁止 rust 对函数名进行“改名（mangling）”，保证生成的链接符号和函数名一致
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    if fork() == 0 {
        exec("user-shell\0", &[core::ptr::null::<u8>()]);
    } else {
        loop {
            let mut exit_code: i32 = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                yield_();
                continue;
            }
            println!(
                "[initproc] Released a zombie process, pid={}, exit_code={}",
                pid, exit_code
            );
            break;
        }
    }
    0
}
