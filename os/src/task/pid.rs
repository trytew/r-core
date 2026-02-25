use alloc::vec::Vec;

///
/// 进程ID
///
/// @author: tryte
///
/// @date: 2026/2/5
pub struct PidHandle(pub usize);

///
/// 进程ID分配器
///
/// @author: tryte
///
/// @date: 2026/2/5
pub struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    ///
    /// 创建进程ID分配器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/5
    pub fn new() -> Self {
        PidAllocator {
            current: 0,
            recycled: Vec::new(),
        }
    }

    ///
    /// 分配进程ID
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/5
    pub fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            PidHandle(pid)
        } else {
            self.current += 1;
            PidHandle(self.current - 1)
        }
    }

    ///
    /// 回收进程ID
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/5
    pub fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.current);
        assert!(
            !self.recycled.iter().any(|ppid| { *ppid == pid }),
            "pid {} has been deallocated!",
            pid
        );
        self.recycled.push(pid);
    }
}

pub struct KernelStack {
    pid: usize,
}
