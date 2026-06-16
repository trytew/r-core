#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use user_lib::{exit, sleep, thread_create, wait_tid};

static mut TURN: usize = 10;
static mut FLAG: [FlagState; THREAD_NUM] = [FlagState::Out; THREAD_NUM];
static GUARD: AtomicUsize = AtomicUsize::new(0);

const N: usize = 2;
const THREAD_NUM: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlagState {
    Out,
    Want,
    In,
}

fn critical_test_enter() {
    assert_eq!(GUARD.fetch_add(1, Ordering::SeqCst), 0);
}

fn critical_test_claim() {
    assert_eq!(GUARD.load(Ordering::SeqCst), 1);
}

fn critical_test_exit() {
    assert_eq!(GUARD.fetch_sub(1, Ordering::SeqCst), 1);
}

fn eisenberg_enter_critical(id: usize) {
    loop {
        println!("Thread[{}] try enter", id);
        v_store!(FLAG[id], FlagState::Want);
        loop {
            let mut prior_thread: Option<usize> = None;
            let turn = v_load!(TURN);
            let ring_id = if id < turn { id + THREAD_NUM } else { id };
            for i in turn..ring_id {
                if v_load!(FLAG[i % THREAD_NUM]) != FlagState::Out {
                    prior_thread = Some(i % THREAD_NUM);
                    break;
                }
            }
            if prior_thread.is_none() {
                break;
            }
            println!(
                "Thread[{}]: prior thread {} exist, sleep and retry",
                id,
                prior_thread.unwrap()
            );
            sleep(1);
        }
        v_store!(FLAG[id], FlagState::In);
        memory_fence!();
        let mut conflict = false;
        for i in 0..THREAD_NUM {
            if i != id && v_load!(FLAG[i]) == FlagState::In {
                conflict = true;
            }
        }
        if !conflict {
            break;
        }
        println!("Thread[{}]: CONFLICT!", id);
    }
    v_store!(TURN, id);
    print!("Thread[{}] enter", id);
}

fn eisenberg_exit_critical(id: usize) {
    let mut next = id;
    let ring_id = id + THREAD_NUM;
    for i in (id + 1)..ring_id {
        let idx = i % THREAD_NUM;
        if v_load!(FLAG[idx]) == FlagState::Want {
            next = idx;
            break;
        }
    }
    v_store!(TURN, next);
    v_store!(FLAG[id], FlagState::Out);
    println!("Thread[{}] exit, give turn to {}", id, next);
}

pub fn thread_fn(id: usize) -> ! {
    println!("Thread[{}] init.", id);
    for _ in 0..N {
        eisenberg_enter_critical(id);
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
    assert_eq!(THREAD_NUM, 10);

    let shuffle: [usize; 10] = [7, 0, 4, 6, 2, 9, 8, 1, 3, 5];
    for i in 0..THREAD_NUM {
        v.push(thread_create(thread_fn as *const () as usize, shuffle[i]));
    }
    for tid in v.iter() {
        let exit_code = wait_tid(*tid as usize);
        assert_eq!(exit_code, 0, "thread conflict happened!");
        println!("thread#{} exited with code {}", tid, exit_code);
    }
    println!("main thread exited.");
    exit(0)
}
