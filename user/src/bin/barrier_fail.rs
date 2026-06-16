#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::vec::Vec;
use user_lib::{exit, thread_create, wait_tid};

const THREAD_NUM: usize = 3;

fn thread_fn() {
    for ch in 'a'..='c' {
        for _ in 0..300 {
            print!("{}", ch);
        }
    }
    exit(0)
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let mut v: Vec<isize> = Vec::new();
    for _ in 0..THREAD_NUM {
        v.push(thread_create(thread_fn as *const () as usize, 0));
    }
    for tid in v.into_iter() {
        wait_tid(tid as usize);
    }
    println!("\nOK!");
    0
}
