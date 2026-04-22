#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exit, fork, wait, waitpid, yield_};

const MAGID: i32 = -0x10384;

///
/// 测试-fork
///
/// @author: tryte
///
/// @date: 2026/3/14
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("I am the parent. Forking the child");
    let pid = fork();
    if pid == 0 {
        println!("I am the child");
        for _ in 0..7 {
            yield_();
        }
        exit(MAGID);
    } else {
        println!("I am parent, fork a child pid {}", pid);
    }
    println!("I am parent, waiting now...");
    let mut x_state = 0;
    assert!(waitpid(pid as usize, &mut x_state) == pid && x_state == MAGID);
    assert!(waitpid(pid as usize, &mut x_state) < 0 && wait(&mut x_state) <= 0);
    println!("x_state: {}", x_state);
    print!("waitpid {} ok.", pid);
    println!("exit pass.");
    0
}
