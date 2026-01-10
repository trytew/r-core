use crate::boards::MEMORY_END;
use crate::mm::address::{PhysAddr, PhysPageNum};
use crate::sync::UpSafeCell;
use alloc::vec::Vec;
use lazy_static::lazy_static;

type FrameAllocatorImpl = StackFrameAllocator;
lazy_static! {
    // 创建全局堆内存分配器
    pub static ref FRAME_ALLOCATOR: UpSafeCell<FrameAllocatorImpl> =
        unsafe { UpSafeCell::new(FrameAllocatorImpl::new()) };
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
    // 空闲内存的起始物理页号
    current: usize,
    // 空闲内存的结束物理页号
    end: usize,
    // 以后入先出的方式保存了被回收的物理页号
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
    }
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

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

    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // 检查页帧是否已被回收
        if ppn >= self.current || self.recycled.iter().find(|&v| *v == ppn).is_some() {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        self.recycled.push(ppn);
    }
}

pub fn init_frame_allocator() {
    unsafe extern "C" {
        fn ekernel();
    }

    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as *const () as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    )
}
