#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use user_lib::{exit, get_time, thread_create, wait_tid, yield_};

static mut A: usize = 0;
static mut PER_THREAD: usize = 0;
static OCCUPIED: AtomicBool = AtomicBool::new(false);

const PER_THREAD_DEFAULT: usize = 1000;
const THREAD_COUNT_DEFAULT: usize = 16;

fn critical_section(t: &mut usize) {
    #[allow(unused_unsafe)]
    let a = unsafe { core::ptr::addr_of_mut!(A) };
    let cur = unsafe { a.read_volatile() };
    for _ in 0..500 {
        *t = (*t) * (*t) % 1007;
    }
    unsafe {
        a.write_volatile(cur + 1);
    }
}

fn lock() {
    // 当 OCCUPIED 是 false 的时候修改成 true，若修改成功则返回 Ok(true)，若比较失败或者修改失败则返回 Err(比较时的值)
    while OCCUPIED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        yield_();
    }
}

fn unlock() {
    OCCUPIED.store(false, Ordering::Relaxed);
}

fn f() -> ! {
    let mut t = 2_usize;
    for _ in 0..unsafe { PER_THREAD } {
        lock();
        critical_section(&mut t);
        unlock();
    }
    exit(t as i32)
}

#[unsafe(no_mangle)]
pub fn main(argc: usize, argv: &[&str]) {
    let mut thread_count = THREAD_COUNT_DEFAULT;
    let mut per_thread = PER_THREAD_DEFAULT;
    if argc > 2 {
        thread_count = argv[1].parse().unwrap();
        if argc >= 3 {
            per_thread = argv[2].parse().unwrap();
        }
    }
    unsafe {
        PER_THREAD = per_thread;
    }

    let start = get_time();
    let mut v = Vec::new();
    for _ in 0..thread_count {
        v.push(thread_create(f as *const () as usize, 0) as usize);
    }
    for tid in v.into_iter() {
        wait_tid(tid);
    }
    println!("time cost is {}ms", get_time() - start);
    assert_eq!(unsafe { A }, unsafe { PER_THREAD } * thread_count);
}
