use crate::config::kernel_stack_position;
use crate::mm::{MapPermission, VirtAddr, KERNEL_SPACE};
use crate::sync::UpSafeCell;
use alloc::vec::Vec;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PID_ALLOCATOR: UpSafeCell<PidAllocator> =
        unsafe { UpSafeCell::new(PidAllocator::new()) };
}

///
/// 进程ID
///
/// @author: tryte
///
/// @date: 2026/2/5
pub struct PidHandle(pub usize);

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}

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

impl KernelStack {
    ///
    /// 创建应用的内核栈
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/5
    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.0;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        KernelStack { pid: pid_handle.0 }
    }

    ///
    /// 获取内核栈栈顶
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/5
    pub fn get_top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.pid);
        kernel_stack_top
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (kernel_stack_bottom, _) = kernel_stack_position(self.pid);
        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
        KERNEL_SPACE
            .exclusive_access()
            .remove_area_with_start_vpn(kernel_stack_bottom_va.into());
    }
}

///
/// 分配进程ID
///
/// @author: tryte
///
/// @date: 2026/3/3
pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.exclusive_access().alloc()
}
