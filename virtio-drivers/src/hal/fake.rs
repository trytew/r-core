use crate::hal::{Hal, PhysAddr, VirtAddr};
use crate::PAGE_SIZE;
use alloc::alloc::{alloc_zeroed, dealloc, handle_alloc_error};
use core::alloc::Layout;

///
/// 虚假硬件抽象层
///
/// @author: tryte
///
/// @date: 2026/6/9
#[derive(Debug)]
pub struct FakeHal;

impl Hal for FakeHal {
    ///
    /// 分配内存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    fn dma_alloc(pages: usize) -> PhysAddr {
        assert_ne!(pages, 0);
        // 以页面布局分配内存
        let layout = Layout::from_size_align(pages * PAGE_SIZE, PAGE_SIZE).unwrap();
        let ptr = unsafe { alloc_zeroed(layout) };
        if ptr.is_null() {
            handle_alloc_error(layout);
        }
        ptr as PhysAddr
    }

    ///
    /// 释放内存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    fn dma_dealloc(p_addr: PhysAddr, pages: usize) -> i32 {
        assert_ne!(pages, 0);
        let layout = Layout::from_size_align(pages * PAGE_SIZE, PAGE_SIZE).unwrap();
        unsafe {
            dealloc(p_addr as *mut u8, layout);
        }
        0
    }

    ///
    /// 物理地址转虚拟地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    fn phys_to_virt(p_addr: PhysAddr) -> VirtAddr {
        p_addr
    }

    ///
    /// 虚拟地址转物理地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
        vaddr
    }
}
