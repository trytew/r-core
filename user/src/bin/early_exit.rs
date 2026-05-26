#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use user_lib::{exit, thread_create};

fn thread_a() -> ! {
    for i in 0..1000 {
        print!("{}", i);
    }
    exit(0)
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    thread_create(thread_a as *const () as usize, 0);
    println!("main thread exited!");
    exit(0)
}
