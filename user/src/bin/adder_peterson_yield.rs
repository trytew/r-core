#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::vec::Vec;
use core::ptr::addr_of_mut;
use core::sync::atomic::{compiler_fence, Ordering};
use user_lib::{exit, get_time, thread_create, wait_tid, yield_};

const PER_THREAD_DEFAULT: usize = 2000;
const THREAD_COUNT_DEFAULT: usize = 2;

static mut A: usize = 0;
static mut FLAG: [bool; 2] = [false; 2];
static mut TURN: usize = 0;
static mut PER_THREAD: usize = 0;

fn critical_section(t: &mut usize) {
    #[allow(unused_unsafe)]
    let a = unsafe { addr_of_mut!(A) };
    let cur = unsafe { a.read_volatile() };
    for _ in 0..500 {
        *t = (*t) * (*t) % 1007;
    }
    unsafe {
        a.write_volatile(cur + 1);
    }
}

fn lock(id: usize) {
    unsafe {
        FLAG[id] = true;
        let j = 1 - id;
        TURN = j;
        // 阻止编译器对内存操作进行重排序
        // 作用是在这句之后的代码保证看到的内存内容都是一致的，不会有指令乱序执行时每个 cpu 指令看到的内存布局不一致
        // 这句代码不代表 cpu 指令不会乱序执行，在 cpu 指令执行过程中不会立马将内存新值刷进内存，CPU有多级缓存，
        // 在计算的过程中会先将值放入缓存，待计算结果完成后才会将新值刷进内存，因此多个变量的内存值可能不会同步更新，如先刷了变量a，
        // 然后变量b因后续有还有计算先放入缓存，在这个过程中就会有多个内存观察者读取的内存不是最新的情况，
        // 这句代码就是保证在它之前的内存值所有观察者读取的都是一致的
        compiler_fence(Ordering::SeqCst);
        // 这里的 TURN 有可能被并发修改，如果 j == 1-id 代表另一个线程取得锁，等待另一个线程 unlock => FLAG[j] = false 则会停止阻塞
        // 这是经典的 Peterson Lock（彼得森算法），它是一个理想模型，它假设：
        //   - 单次读写是原子的
        //   - 内存可见性正确
        //   - 不会乱序
        while v_load!(FLAG[j]) && v_load!(TURN) == j {
            yield_();
        }
    }
}

fn unlock(id: usize) {
    unsafe {
        FLAG[id] = false;
    }
}

fn f(id: usize) -> ! {
    let mut t = 2_usize;
    for _iter in 0..unsafe { PER_THREAD } {
        lock(id);
        critical_section(&mut t);
        unlock(id);
    }
    exit(t as i32);
}

#[unsafe(no_mangle)]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    let mut thread_count = THREAD_COUNT_DEFAULT;
    let mut per_thread = PER_THREAD_DEFAULT;
    if argc >= 2 {
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

    assert_eq!(
        thread_count, 2,
        "Peterson works when there are only 2 threads."
    );

    for id in 0..thread_count {
        v.push(thread_create(f as *const () as usize, id) as usize);
    }
    let mut time_cost = Vec::new();
    for tid in v.iter() {
        time_cost.push(wait_tid(*tid));
    }
    println!("time cost is {}ms", get_time() - start);
    assert_eq!(unsafe { A }, unsafe { PER_THREAD } * thread_count);
    0
}
