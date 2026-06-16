#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use user_lib::{exit, sleep, thread_create};

fn thread_a() -> ! {
    println!("into thread_a");
    sleep(1000);
    println!("exit thread_a");
    exit(1)
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    thread_create(thread_a as *const () as usize, 0);
    sleep(100);
    println!("main thread exited!");
    exit(0)
}
