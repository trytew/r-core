#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{fork, getpid, wait};

///
/// 测试-fork正常用法
///
/// @author: tryte
///
/// @date: 2026/3/14
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    assert_eq!(wait(&mut 0_i32), -1);
    println!("sys_wait without child process test passed!");
    println!("parent start, pid = {}", getpid());

    let pid = fork();
    if pid == 0 {
        println!("hello child process!");
        100
    } else {
        let mut exit_code = 0;
        println!("ready waiting on parent process!");
        assert_eq!(pid, wait(&mut exit_code));
        assert_eq!(exit_code, 100);
        println!("child process pid = {}, exit code = {}", pid, exit_code);
        0
    }
}
