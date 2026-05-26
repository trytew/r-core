#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use core::pin::Pin;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

/// 构造一个空数据的 trait 实现
const RAW_WAKER: RawWaker = RawWaker::new(core::ptr::null(), &VTABLE);

enum State {
    Halted,
    Running,
}

struct Task {
    state: State,
}

impl Task {
    fn waiter<'a>(&'a mut self) -> Waiter<'a> {
        Waiter { task: self }
    }
}

struct Waiter<'a> {
    task: &'a mut Task,
}

impl<'a> Future for Waiter<'a> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
        println!("hh");
        match self.task.state {
            State::Halted => {
                self.task.state = State::Running;
                Poll::Ready(())
            }
            State::Running => {
                self.task.state = State::Halted;
                Poll::Pending
            }
        }
    }
}

struct Executor {
    task: VecDeque<Pin<Box<dyn Future<Output = ()>>>>,
}

impl Executor {
    fn new() -> Self {
        Executor {
            task: VecDeque::new(),
        }
    }

    fn push<C, F>(&mut self, closure: C)
    where
        F: Future<Output = ()> + 'static,
        C: FnOnce(Task) -> F,
    {
        let task = Task {
            state: State::Running,
        };
        self.task.push_back(Box::pin(closure(task)))
    }

    fn run(&mut self) {
        let waker = create_waker();
        let mut context = Context::from_waker(&waker);

        while let Some(mut task) = self.task.pop_front() {
            match task.as_mut().poll(&mut context) {
                Poll::Ready(()) => {}
                Poll::Pending => {
                    self.task.push_back(task);
                }
            }
        }
    }
}

pub fn create_waker() -> Waker {
    unsafe { Waker::from_raw(RAW_WAKER) }
}

unsafe fn clone(_: *const ()) -> RawWaker {
    RAW_WAKER
}

unsafe fn wake(_: *const ()) {}

unsafe fn wake_by_ref(_: *const ()) {}

unsafe fn drop(_: *const ()) {}

///
/// 无栈协程
///
/// @author: tryte
///
/// @date: 2026/5/25
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("stackless_coroutine Begin...");

    let mut exec = Executor::new();
    println!("  Create futures");

    for instance in 1..=3 {
        exec.push(move |mut task| async move {
            println!("      Task {}: begin state", instance);
            task.waiter().await;
            println!("      Task {}: next state", instance);
            task.waiter().await;
            println!("      Task {}: end state", instance);
        })
    }

    println!("  Running");
    exec.run();
    println!("  Done");
    println!("stackless_coroutine PASSED");
    0
}
