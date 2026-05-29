use crate::sync::{Mutex, UpSafeCell};
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::collections::VecDeque;
use alloc::sync::Arc;

pub struct CondVarInner {
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

pub struct CondVar {
    pub inner: UpSafeCell<CondVarInner>,
}

impl CondVar {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UpSafeCell::new(CondVarInner {
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    ///
    /// 条件已满足，唤醒线程
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/29
    pub fn signal(&self) {
        let mut inner = self.inner.exclusive_access();
        if let Some(task) = inner.wait_queue.pop_front() {
            wakeup_task(task);
        }
    }

    ///
    /// 等待条件变量满足要求
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/29
    pub fn wait(&self, mutex: Arc<dyn Mutex>) {
        mutex.unlock();
        let mut inner = self.inner.exclusive_access();
        inner.wait_queue.push_back(current_task().unwrap());
        drop(inner);
        block_current_and_run_next();
        mutex.lock();
    }
}
