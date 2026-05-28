use crate::sync::UpSafeCell;
use crate::task::{
    block_current_and_run_next, current_task, suspend_current_and_run_next, wakeup_task,
    TaskControlBlock,
};
use alloc::collections::VecDeque;
use alloc::sync::Arc;

pub trait Mutex: Sync + Send {
    fn lock(&self);
    fn unlock(&self);
}

pub struct MutexSpin {
    locked: UpSafeCell<bool>,
}

impl MutexSpin {
    pub fn new() -> Self {
        Self {
            locked: unsafe { UpSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    ///
    /// 上锁
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/28
    fn lock(&self) {
        // 上锁
        loop {
            // 如果无法持有锁则切换到下一个线程运行
            let mut locked = self.locked.exclusive_access();
            if *locked {
                // 提前释放 locked，因为 locked 是一个 RefMut<>，如果线程切走还一直处于持有状态的话当下个线程尝试获取时就会 panic
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                return;
            }
        }
    }

    ///
    /// 解锁
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/28
    fn unlock(&self) {
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

pub struct MutexBlocking {
    inner: UpSafeCell<MutexBlockingInner>,
}

impl MutexBlocking {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UpSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    ///
    /// 上锁
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/28
    fn lock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        // 如果无法持有锁则将当前线程设置成阻塞态并放入锁等待队列等待锁获取
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
        }
    }

    ///
    /// 解锁
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/28
    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}
