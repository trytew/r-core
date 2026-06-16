#![no_std]
#![no_main]
#![allow(clippy::empty_loop)]

#[macro_use]
#[allow(unused)]
extern crate user_lib;

///
/// 死循环
///
/// @author: tryte
///
/// @date: 2026/3/16
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    loop {}
}
