use crate::sync::{Mutex, UpIntrFreeCell};
use crate::task::{
    block_current_and_run_next, block_current_task, current_task, wakeup_task, TaskContext,
    TaskControlBlock,
};
use alloc::collections::VecDeque;
use alloc::sync::Arc;

pub struct CondVarInner {
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

pub struct CondVar {
    pub inner: UpIntrFreeCell<CondVarInner>,
}

impl CondVar {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UpIntrFreeCell::new(CondVarInner {
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

    ///
    /// 阻塞当前线程
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    pub fn wait_no_sched(&self) -> *mut TaskContext {
        self.inner.exclusive_session(|inner| {
            inner.wait_queue.push_back(current_task().unwrap());
        });
        block_current_task()
    }

    pub fn wait_with_mutex(&self, mutex: Arc<dyn Mutex>) {
        mutex.unlock();
        self.inner.exclusive_session(|inner| {
            inner.wait_queue.push_back(current_task().unwrap());
        });
        block_current_and_run_next();
        mutex.lock();
    }
}
