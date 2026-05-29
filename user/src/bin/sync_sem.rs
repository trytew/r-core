#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::vec;
use user_lib::{
    exit, semaphore_create, semaphore_down, semaphore_up, sleep, thread_create, wait_tid,
};

const SEM_SYNC: usize = 0;

fn first() -> ! {
    sleep(10);
    println!("First work and wakeup Second");
    semaphore_up(SEM_SYNC);
    exit(0);
}

fn second() -> ! {
    println!("Second want to continue, but need to wait first!");
    semaphore_down(SEM_SYNC);
    println!("Second can work now!");
    exit(0);
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    assert_eq!(semaphore_create(0) as usize, SEM_SYNC);

    let threads = vec![
        thread_create(first as *const () as usize, 0),
        thread_create(second as *const () as usize, 0),
    ];

    for thread in threads.iter() {
        wait_tid(*thread as usize);
    }
    0
}
