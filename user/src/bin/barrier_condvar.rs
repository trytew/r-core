#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::vec::Vec;
use core::cell::UnsafeCell;
use lazy_static::*;
use user_lib::{
    condvar_create, condvar_signal, condvar_wait, exit, mutex_create, mutex_lock, mutex_unlock,
    thread_create, wait_tid,
};

lazy_static! {
    static ref BARRIER_AB: Barrier = Barrier::new();
    static ref BARRIER_BC: Barrier = Barrier::new();
}

const THREAD_NUM: usize = 3;

struct Barrier {
    mutex_id: usize,
    condvar_id: usize,
    count: UnsafeCell<usize>,
}

impl Barrier {
    pub fn new() -> Self {
        Self {
            mutex_id: mutex_create() as usize,
            condvar_id: condvar_create() as usize,
            count: UnsafeCell::new(0),
        }
    }

    pub fn block(&self) {
        mutex_lock(self.mutex_id);
        let count = self.count.get();
        unsafe {
            *count = *count + 1;
        }
        if unsafe { *count } == THREAD_NUM {
            condvar_signal(self.condvar_id);
        } else {
            // 这里当等待结束后还调用一次 condvar_signal 的原因是操作系统实现的 condvar_signal 并不是一次性唤醒所有线程的，
            // 而是需要上一个唤醒下一个这种传播式唤醒，同时这也不是标准的条件变量使用方式，因为很容易漏掉这一句，只是教学版简化了同时唤醒的复杂性
            condvar_wait(self.condvar_id, self.mutex_id);
            condvar_signal(self.condvar_id);
        }
        mutex_unlock(self.mutex_id);
    }
}

unsafe impl Sync for Barrier {}

fn thread_fn() {
    for _ in 0..300 {
        print!("a");
    }
    BARRIER_AB.block();
    for _ in 0..300 {
        print!("b");
    }
    BARRIER_BC.block();
    for _ in 0..300 {
        print!("c");
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
