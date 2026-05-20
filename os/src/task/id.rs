use crate::config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT_BASE, USER_STACK_SIZE};
use crate::mm::{MapPermission, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::sync::UpSafeCell;
use crate::task::process::ProcessControlBlock;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;
use lazy_static::lazy_static;

lazy_static! {
    // 进程ID分配器
    static ref PID_ALLOCATOR: UpSafeCell<RecycleAllocator> =
        unsafe { UpSafeCell::new(RecycleAllocator::new()) };

    // 内核栈ID分配器
    static ref KERNEL_STACK_ALLOCATOR: UpSafeCell<RecycleAllocator> =
        unsafe { UpSafeCell::new(RecycleAllocator::new()) };
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
/// 可回收ID分配器
///
/// @author: tryte
///
/// @date: 2026/2/5
pub struct RecycleAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl RecycleAllocator {
    ///
    /// 创建可回收ID分配器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/5
    pub fn new() -> Self {
        RecycleAllocator {
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
    pub fn alloc(&mut self) -> usize {
        if let Some(id) = self.recycled.pop() {
            id
        } else {
            self.current += 1;
            self.current - 1
        }
    }

    ///
    /// 回收进程ID
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/5
    pub fn dealloc(&mut self, id: usize) {
        assert!(id < self.current);
        assert!(
            !self.recycled.iter().any(|i| { *i == id }),
            "id {} has been deallocated!",
            id
        );
        self.recycled.push(id);
    }
}

pub struct KernelStack(pub usize);

impl KernelStack {
    #[allow(unused)]
    ///
    /// 将值放到内核栈栈顶
    ///
    /// @author: tryte
    ///
    /// @date: 2026/4/7
    pub fn push_to_top<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {
        let kernel_stack_top = self.get_top();
        let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *ptr_mut = value;
        }
        ptr_mut
    }

    ///
    /// 获取内核栈栈顶
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/5
    pub fn get_top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.0);
        kernel_stack_top
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        // 获取内核栈位置
        let (kernel_stack_bottom, _) = kernel_stack_position(self.0);
        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();

        // 删除内核栈所在内存
        KERNEL_SPACE
            .exclusive_access()
            .remove_area_with_start_vpn(kernel_stack_bottom_va.into());

        // 回收使用ID
        KERNEL_STACK_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}

///
/// 线程的用户态资源
///
/// @author: tryte
///
/// @date: 2026/5/20
pub struct TaskUserResource {
    /// 线程ID
    pub tid: usize,
    // 用户栈地址（虚拟地址）
    pub user_stack_base: usize,
    /// 所属进程
    pub process: Weak<ProcessControlBlock>,
}

impl TaskUserResource {
    ///
    /// 创建线程用户态资源
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/20
    pub fn new(
        process: Arc<ProcessControlBlock>,
        user_stack_base: usize,
        alloc_user_res: bool,
    ) -> Self {
        // 生成线程ID
        let tid = process.inner_exclusive_access().alloc_tid();

        let task_user_res = Self {
            tid,
            user_stack_base,
            process: Arc::downgrade(&process),
        };
        if alloc_user_res {
            // 分配用户资源内存
            task_user_res.alloc_user_res();
        }
        task_user_res
    }

    ///
    /// 分配用户资源
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/20
    pub fn alloc_user_res(&self) {
        // 获取进程
        let process = self.process.upgrade().unwrap();
        let mut process_inner = process.inner_exclusive_access();
        // 获取用户栈位置
        let user_stack_bottom = user_stack_bottom_from_tid(self.user_stack_base, self.tid);
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        // 分配用户栈空间（从进程已使用内存+灰页后开始）
        process_inner.memory_set.insert_framed_area(
            user_stack_bottom.into(),
            user_stack_top.into(),
            MapPermission::R | MapPermission::W | MapPermission::U,
        );
        // 获取“陷入”上下文地址（从“跳板”地址往下 tid * PAGE_SIZE 开始）
        let trap_cx_bottom = trap_cx_bottom_from_tid(self.tid);
        let trap_cx_top = trap_cx_bottom + PAGE_SIZE;
        // 分配“陷入”上下文内存
        process_inner.memory_set.insert_framed_area(
            trap_cx_bottom.into(),
            trap_cx_top.into(),
            MapPermission::R | MapPermission::W,
        );
    }

    fn dealloc_user_res(&self) {
        let process = self.process.upgrade().unwrap();
        let mut process_inner = process.inner_exclusive_access();
        let user_stack_bottom_va: VirtAddr =
            user_stack_bottom_from_tid(self.user_stack_base, self.tid).into();

        process_inner
            .memory_set
            .remove_area_with_start_vpn(user_stack_bottom_va.into());

        let trap_cx_bottom_va: VirtAddr = trap_cx_bottom_from_tid(self.tid).into();
        process_inner
            .memory_set
            .remove_area_with_start_vpn(trap_cx_bottom_va.into());
    }

    #[allow(unused)]
    pub fn alloc_tid(&mut self) {
        self.tid = self
            .process
            .upgrade()
            .unwrap()
            .inner_exclusive_access()
            .alloc_tid();
    }

    pub fn dealloc_tid(&self) {
        let process = self.process.upgrade().unwrap();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.dealloc_tid(self.tid);
    }

    pub fn trap_cx_user_va(&self) -> usize {
        trap_cx_bottom_from_tid(self.tid)
    }

    ///
    /// 获取“陷入”上下文物理地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/5/20
    pub fn trap_cx_ppn(&self) -> PhysPageNum {
        // 获取进程
        let process = self.process.upgrade().unwrap();
        let process_inner = process.inner_exclusive_access();
        // 计算“陷入”上下文起始地址
        let trap_cx_bottom_va: VirtAddr = trap_cx_bottom_from_tid(self.tid).into();
        process_inner
            .memory_set
            .translate(trap_cx_bottom_va.into())
            .unwrap()
            .ppn()
    }

    pub fn user_stack_base(&self) -> usize {
        self.user_stack_base
    }

    pub fn user_stack_top(&self) -> usize {
        user_stack_bottom_from_tid(self.user_stack_base, self.tid) + USER_STACK_SIZE
    }
}

impl Drop for TaskUserResource {
    fn drop(&mut self) {
        self.dealloc_tid();
        self.dealloc_user_res();
    }
}

///
/// 分配进程ID
///
/// @author: tryte
///
/// @date: 2026/3/3
pub fn pid_alloc() -> PidHandle {
    PidHandle(PID_ALLOCATOR.exclusive_access().alloc())
}

///
/// 获取进程内核栈底和栈顶
///
/// @author: tryte
///
/// @date: 2026/1/30
pub fn kernel_stack_position(kernel_stack_id: usize) -> (usize, usize) {
    // 内核栈的空间是8kb + 4kb，其中有 4kb 是作为灰页（保护页）不映射进页表
    let top = TRAMPOLINE - kernel_stack_id * (KERNEL_STACK_SIZE + PAGE_SIZE); // 高地址
    let bottom = top - KERNEL_STACK_SIZE; // 低地址
    (bottom, top)
}

///
/// 创建内核栈
///
/// @author: tryte
///
/// @date: 2026/5/19
pub fn kernel_stack_alloc() -> KernelStack {
    // 生成内核栈id
    let kernel_stack_id = KERNEL_STACK_ALLOCATOR.exclusive_access().alloc();
    // 计算内核栈位置
    let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(kernel_stack_id);
    // 分配内核栈空间，这里用的是内核虚拟空间，与进程内存空间不重叠，因此不会影响进程内存空间
    KERNEL_SPACE.exclusive_access().insert_framed_area(
        kernel_stack_bottom.into(),
        kernel_stack_top.into(),
        MapPermission::R | MapPermission::W,
    );
    KernelStack(kernel_stack_id)
}

///
/// 根据线程id获取“陷入”上下文地址
///
/// @author: tryte
///
/// @date: 2026/5/19
fn trap_cx_bottom_from_tid(tid: usize) -> usize {
    TRAP_CONTEXT_BASE - tid * PAGE_SIZE
}

///
/// 获取用户栈栈底
///
/// @author: tryte
///
/// @date: 2026/5/19
fn user_stack_bottom_from_tid(user_stack_base: usize, tid: usize) -> usize {
    user_stack_base + tid * (PAGE_SIZE + USER_STACK_SIZE)
}
