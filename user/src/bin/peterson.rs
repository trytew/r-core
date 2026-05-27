#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use user_lib::{exit, sleep, thread_create, wait_tid};

static mut TURN: usize = 10;
static mut FLAG: [bool; 2] = [false; 2];
static GUARD: AtomicUsize = AtomicUsize::new(0);

const N: usize = 1000;

fn critical_test_enter() {
    assert_eq!(GUARD.fetch_add(1, Ordering::SeqCst), 0);
}

fn critical_test_claim() {
    assert_eq!(GUARD.load(Ordering::SeqCst), 1);
}

fn critical_test_exit() {
    assert_eq!(GUARD.fetch_sub(1, Ordering::SeqCst), 1);
}

fn eisenberg_enter_critical(id: usize, peer_id: usize) {
    // println!("Thread[{}] try enter", id);
    v_store!(FLAG[id], true);
    v_store!(TURN, peer_id);
    memory_fence!();
    while v_load!(FLAG[peer_id]) && v_load!(TURN) == peer_id {
        // println!("Thread[{}] enter fail", id);
        sleep(1);
        // println!("Thread[{}] retry enter", id);
    }
    // println!("Thread[{}] enter", id);
}

fn eisenberg_exit_critical(id: usize) {
    v_store!(FLAG[id], false);
    // println!("Thread[{}] exit", id);
}

pub fn thread_fn(id: usize) -> ! {
    // println!("Thread[{}] init.", id);
    let peer_id: usize = id ^ 1;
    for iter in 0..N {
        if iter % 10 == 0 {
            println!("[{}] it = {}", id, iter);
        }
        eisenberg_enter_critical(id, peer_id);
        critical_test_enter();
        for _ in 0..3 {
            critical_test_claim();
            sleep(2);
        }
        critical_test_exit();
        eisenberg_exit_critical(id);
    }
    exit(0)
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let mut v = Vec::new();
    v.push(thread_create(thread_fn as *const () as usize, 0));
    v.push(thread_create(thread_fn as *const () as usize, 1));
    for tid in v.iter() {
        let exit_code = wait_tid(*tid as usize);
        assert_eq!(exit_code, 0, "thread conflict happened!");
        println!("thread#{} exited with code {}", tid, exit_code);
    }
    println!("main thread exited.");
    exit(0)
}
