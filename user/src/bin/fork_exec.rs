#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, getpid, wait};

///
/// 创建子进程并执行新程序
///
/// @author: tryte
///
/// @date: 2026/3/14
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("pid {}: parent start forking...", getpid());

    let pid = fork();
    if pid == 0 {
        println!(
            "pid {}: forked child start executing hello_world app...",
            getpid()
        );
        exec("hello_world\0");
        100
    } else {
        let mut exit_code = 0;
        println!("pid {}: ready waiting child...", getpid());
        assert_eq!(pid, wait(&mut exit_code));
        assert_eq!(exit_code, 0);
        println!(
            "pid {}: got child info:: pid {}, exit_code: {}",
            getpid(),
            pid,
            exit_code
        );
        0
    }
}
