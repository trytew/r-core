pub(crate) mod fake;

use crate::Result;
use crate::{Error, PAGE_SIZE};
use core::marker::PhantomData;

/// 虚拟地址类型别名
pub type VirtAddr = usize;

/// 物理地址类型别名
pub type PhysAddr = usize;

///
/// 硬件抽象层（Hardware Abstract Layer）
///
/// @author: tryte
///
/// @date: 2026/6/8
pub trait Hal {
    ///
    /// 分配 DMA 内存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/8
    fn dma_alloc(pages: usize) -> PhysAddr;

    ///
    /// 释放 DMA 内存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/8
    fn dma_dealloc(p_addr: PhysAddr, pages: usize) -> i32;

    ///
    /// 物理地址转虚拟地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/8
    fn phys_to_virt(p_addr: PhysAddr) -> VirtAddr;

    ///
    /// 虚拟地址转物理地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/8
    fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr;
}

///
/// 直接内存访问（Direct Memory Access）
///
/// @author: tryte
///
/// @date: 2026/6/8
#[derive(Debug)]
pub struct DMA<H: Hal> {
    /// 物理地址
    p_addr: usize,
    /// 使用内存页数
    pages: usize,
    /// 幽灵数据，实际不占内存空间，只是为了让 DMA 持有 H 泛型
    /// 幽灵数据还有一个作用是可以让类型系统更深层判断类型，如：DMA<VirtioHal> != DMA<TestHal>
    _phantom: PhantomData<H>,
}

impl<H: Hal> DMA<H> {
    ///
    /// 实例化 DMA
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn new(pages: usize) -> Result<Self> {
        let p_addr = H::dma_alloc(pages);
        if p_addr == 0 {
            return Err(Error::DmaError);
        }
        Ok(DMA {
            p_addr,
            pages,
            _phantom: PhantomData::default(),
        })
    }

    pub fn p_addr(&self) -> usize {
        self.p_addr
    }

    pub fn vaddr(&self) -> usize {
        H::phys_to_virt(self.p_addr)
    }

    ///
    /// 获取 DMA 物理内存页号
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn pfn(&self) -> u32 {
        (self.p_addr >> 12) as u32
    }

    ///
    /// 将内容当 u8 数组输出
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/13
    pub unsafe fn as_buf(&self) -> &'static mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.vaddr() as _, PAGE_SIZE * self.pages) }
    }
}

impl<H: Hal> Drop for DMA<H> {
    fn drop(&mut self) {
        let err = H::dma_dealloc(self.p_addr, self.pages);
        assert_eq!(err, 0, "failed to deallocate DMA");
    }
}
