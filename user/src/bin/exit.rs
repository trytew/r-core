#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exit, fork, wait, waitpid, yield_};

const MAGIC: i32 = -0x10384;

///
/// Shell 退出程序
///
/// @author: tryte
///
/// @date: 2026/3/10
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("I am the parent. Forking the child...");

    let pid = fork();
    if pid == 0 {
        println!("I am the child");
        for _ in 0..7 {
            yield_();
        }
        exit(MAGIC);
    } else {
        println!("I am parent, fork a child pid {}", pid);
    }

    println!("I am the parent, waiting now...");

    let mut x_state: i32 = 0;
    assert!(waitpid(pid as usize, &mut x_state) == pid && x_state == MAGIC);
    assert!(waitpid(pid as usize, &mut x_state) < 0 && wait(&mut x_state) <= 0);
    println!("waitpid {} ok.", pid);
    println!("exit pass.");

    0
}
