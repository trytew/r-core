#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::vec;
use user_lib::{
    condvar_create, condvar_signal, condvar_wait, exit, mutex_blocking_create, mutex_lock,
    mutex_unlock, sleep, thread_create, wait_tid,
};

static mut A: usize = 0;

const CONDVAR_ID: usize = 0;
const MUTEX_ID: usize = 0;

fn first() -> ! {
    sleep(10);
    println!("First work, Change A --> 1 and wakeup Second");
    mutex_lock(MUTEX_ID);
    unsafe {
        A = 1;
    }
    condvar_signal(CONDVAR_ID);
    mutex_unlock(MUTEX_ID);
    exit(0);
}

fn second() -> ! {
    println!("Second want to continue, but need to wait A=1");
    mutex_lock(MUTEX_ID);

    while unsafe { A } == 0 {
        println!("Second: A is {}", unsafe { A });
        condvar_wait(CONDVAR_ID, MUTEX_ID);
    }

    println!("A is {}, Second can work now", unsafe { A });
    mutex_unlock(MUTEX_ID);
    exit(0);
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    assert_eq!(condvar_create() as usize, CONDVAR_ID);
    assert_eq!(mutex_blocking_create() as usize, MUTEX_ID);

    let threads = vec![
        thread_create(first as *const () as usize, 0),
        thread_create(second as *const () as usize, 0),
    ];

    for thread in threads.iter() {
        wait_tid(*thread as usize);
    }
    println!("test_condvar passed!");
    0
}
