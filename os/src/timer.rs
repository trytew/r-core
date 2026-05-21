use crate::boards::CLOCK_FREQ;
use crate::sbi::set_timer;
use crate::sync::UpSafeCell;
use crate::task::{wakeup_task, TaskControlBlock};
use alloc::collections::BinaryHeap;
use alloc::sync::Arc;
use core::cmp::Ordering;
use lazy_static::lazy_static;
use riscv::register::time;

lazy_static! {
    static ref TIMERS: UpSafeCell<BinaryHeap<TimerCondVar>> =
        unsafe { UpSafeCell::new(BinaryHeap::<TimerCondVar>::new()) };
}

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;

///
/// 定时器
///
/// @author: tryte
///
/// @date: 2026/5/21
pub struct TimerCondVar {
    pub expire_ms: usize,
    pub task: Arc<TaskControlBlock>,
}

impl PartialEq for TimerCondVar {
    fn eq(&self, other: &Self) -> bool {
        self.expire_ms == other.expire_ms
    }
}

impl Eq for TimerCondVar {}

impl PartialOrd for TimerCondVar {
    ///
    /// 比较定时器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/21
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // 过期时间取反，因此过期时间最小的最大
        let a = -(self.expire_ms as isize);
        let b = -(other.expire_ms as isize);
        Some(a.cmp(&b))
    }
}

impl Ord for TimerCondVar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

///
/// 获取时间
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn get_time() -> usize {
    time::read()
}

///
/// 获取毫秒
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

///
/// 设置下一个定时中断
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn set_next_tigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

///
/// 添加定时器
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn add_timer(expire_ms: usize, task: Arc<TaskControlBlock>) {
    let mut timers = TIMERS.exclusive_access();
    timers.push(TimerCondVar { expire_ms, task })
}

///
/// 从定时器队列中移除线程
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn remove_timer(task: Arc<TaskControlBlock>) {
    let mut timers = TIMERS.exclusive_access();
    let mut temp = BinaryHeap::<TimerCondVar>::new();
    for condvar in timers.drain() {
        if Arc::as_ptr(&task) != Arc::as_ptr(&condvar.task) {
            temp.push(condvar);
        }
    }
    timers.clear();
    timers.append(&mut temp);
}

///
/// 查看是否已到时间
///
/// @author: tryte
///
/// @date: 2026/5/21
pub fn check_timer() {
    // 获取当前时间
    let current_ms = get_time_ms();
    let mut timers = TIMERS.exclusive_access();
    // 从过期时间最小的定时器开始遍历，因为定时器实现了 Ord Trait，比较时时间取反后再比较，因此过期时间最小的定时器值最大
    while let Some(timer) = timers.peek() {
        if timer.expire_ms <= current_ms {
            // 过期，唤醒线程
            wakeup_task(Arc::clone(&timer.task));
            // 定时器弹出
            timers.pop();
        } else {
            break;
        }
    }
}
