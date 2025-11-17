#![no_std]
#![no_main]

mod lang_items;

use core::arch::global_asm;

// 加载入口汇编文件
global_asm!(include_str!("./entry.asm"));

// fn main() {
//     // println!("Hello, world!");
// }
