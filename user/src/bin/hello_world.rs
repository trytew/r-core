#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::getpid;

///
/// 打印 hello world
///
/// @author: tryte
///
/// @date: 2026/3/10
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("pid {}: Hello world from user mode program!", getpid());
    0
}
