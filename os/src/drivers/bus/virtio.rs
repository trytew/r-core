use crate::mm::{
    frame_alloc_more, frame_dealloc, kernel_token, FrameTracker, PageTable, PhysAddr, PhysPageNum,
    StepByOne, VirtAddr,
};
use crate::sync::UpIntrFreeCell;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use virtio_drivers::Hal;

lazy_static! {
    static ref QUEUE_FRAMES: UpIntrFreeCell<Vec<FrameTracker>> =
        unsafe { UpIntrFreeCell::new(Vec::new()) };
}

///
/// 硬件抽象层
///
/// @author: tryte
///
/// @date: 2026/6/9
pub struct VirtioHal;

impl Hal for VirtioHal {
    ///
    /// 分配内存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    fn dma_alloc(pages: usize) -> usize {
        let tracers = frame_alloc_more(pages);
        let ppn_base = tracers.as_ref().unwrap().last().unwrap().ppn;
        QUEUE_FRAMES
            .exclusive_access()
            .append(&mut tracers.unwrap());
        let pa: PhysAddr = ppn_base.into();
        pa.0
    }

    ///
    /// 释放内存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    fn dma_dealloc(p_addr: virtio_drivers::PhysAddr, pages: usize) -> i32 {
        let pa = PhysAddr::from(p_addr);
        let mut ppn_base: PhysPageNum = pa.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }
        0
    }

    ///
    /// 物理地址转虚拟地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    fn phys_to_virt(p_addr: virtio_drivers::PhysAddr) -> virtio_drivers::VirtAddr {
        // 因为内核的内存地址是恒等映射，因此物理地址=虚拟地址
        p_addr
    }

    ///
    /// 虚拟地址转物理地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    fn virt_to_phys(vaddr: virtio_drivers::VirtAddr) -> virtio_drivers::PhysAddr {
        // 切换到内核内存空间
        PageTable::from_token(kernel_token())
            // 翻译虚拟地址
            .translate_va(VirtAddr::from(vaddr))
            .unwrap()
            .0
    }
}
