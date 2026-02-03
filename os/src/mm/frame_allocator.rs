use crate::boards::MEMORY_END;
use crate::mm::address::PhysAddr;
use crate::mm::address::PhysPageNum;
use crate::println;
use crate::sync::UpSafeCell;
use alloc::vec::Vec;
use core::fmt::Debug;
use core::fmt::Formatter;
use lazy_static::lazy_static;

type FrameAllocatorImpl = StackFrameAllocator;
lazy_static! {
    // 创建全局物理页帧管理器
    pub static ref FRAME_ALLOCATOR: UpSafeCell<FrameAllocatorImpl> =
        unsafe { UpSafeCell::new(FrameAllocatorImpl::new()) };
}

///
/// 帧追踪器
///
/// @author: tryte
///
/// @date: 2026/1/12
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    ///
    /// 初始化页帧空间
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/12
    pub fn new(ppn: PhysPageNum) -> Self {
        let bytes_array = ppn.get_bytes_array();
        for i in bytes_array {
            *i = 0;
        }
        Self { ppn }
    }
}

impl Debug for FrameTracker {
    ///
    /// 打印页表地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/12
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("FrameTracker: PPN={:#x}", self.ppn.0))
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

///
/// 物理页帧管理器
///
/// @author: tryte
///
/// @date: 2026/1/9
trait FrameAllocator {
    ///
    /// 实例化
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/9
    fn new() -> Self;

    ///
    /// 分配
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/9
    fn alloc(&mut self) -> Option<PhysPageNum>;

    ///
    /// 回收
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/9
    fn dealloc(&mut self, ppn: PhysPageNum);
}

///
/// 栈式物理页帧管理器
///
/// @author: tryte
///
/// @date: 2026/1/9
pub struct StackFrameAllocator {
    // 当前物理页号
    current: usize,
    // 结束物理页号
    end: usize,
    // 以后入先出的方式保存了被回收的物理页号
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    ///
    /// 初始化栈式物理页帧管理器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/21
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        // 设置页帧的物理起始和结束页号
        self.current = l.0;
        self.end = r.0;
    }
}

impl FrameAllocator for StackFrameAllocator {
    ///
    /// 实例化页帧分配器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/22
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    ///
    /// 分配物理页帧
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/22
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            // 优先从已回收的页帧中重新分配
            Some(ppn.into())
        } else {
            if self.current == self.end {
                // 内存已耗尽
                None
            } else {
                // 使用新的页帧，空闲页帧号增加一位
                self.current += 1;
                Some((self.current - 1).into())
            }
        }
    }

    ///
    /// 回收物理帧
    ///
    /// @author: tryte
    ///
    /// @date: 2026/2/3
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // 检查页帧是否已被回收
        if ppn >= self.current || self.recycled.iter().find(|&v| *v == ppn).is_some() {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        println!("ppn dealloc: {:#x}", ppn);
        self.recycled.push(ppn);
    }
}

///
/// 初始化物理页帧管理器
///
/// @author: tryte
///
/// @date: 2026/1/12
pub fn init_frame_allocator() {
    unsafe extern "C" {
        fn ekernel();
    }

    // 设置物理内存空间大小
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as *const () as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    )
}

///
/// 分配内存页
///
/// @author: tryte
///
/// @date: 2026/1/12
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
}

///
/// 回收内存页
///
/// @author: tryte
///
/// @date: 2026/1/12
pub fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

///
/// 测试内存页创建和回收
///
/// @author: tryte
///
/// @date: 2026/1/12
#[allow(unused)]
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();

    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        // 注释下一句后每次循环结束内存页立马回收，因为生命周期结束
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}
