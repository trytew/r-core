use crate::mm::address::PhysPageNum;
use crate::mm::address::VirtPageNum;
use crate::mm::frame_allocator::FrameTracker;
use crate::mm::frame_allocator::frame_alloc;
use crate::mm::memory_set::MapType::Framed;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

///
/// 页表项，用来查找物理内存地址的内容，它存在于页表页（记录页表索引内容的页表）中，结构如下：
///
/// ```
/// |63      54|53    28|27    19|18    10|9   8|7|6|5|4|3|2|1|0|
/// | Reserved | PPN[2] | PPN[1] | PPN[0] | RSW |D|A|G|U|X|W|R|V|
/// |    10    |   26   |    9   |    9   |  2  |1|1|1|1|1|1|1|1|
/// ```
///
/// 字段说明：
/// - V(Valid)：仅当位 V 为 1 时，页表项才是合法的；
/// - R(Read)/W(Write)/X(eXecute)：分别控制索引到这个页表项的对应虚拟页面是否允许读/写/执行；
/// - U(User)：控制索引到这个页表项的对应虚拟页面是否在 CPU 处于 U 特权级的情况下是否被允许访问；
/// - G：暂且不理会；
/// - A(Accessed)：处理器记录自从页表项上的这一位被清零之后，页表项的对应虚拟页面是否被访问过；
/// - D(Dirty)：处理器记录自从页表项上的这一位被清零之后，页表项的对应虚拟页面是否被修改过。
///
/// > 除了 G 外的上述位可以被操作系统设置，只有 A 位和 D 位会被处理器动态地直接设置为 1 ，表示对应的页被访问过或修过（ 注：A 位和 D 位能否被处理器硬件直接修改，取决于处理器的具体实现）。
///
///
/// 使用流程：
///
/// 1. 39位虚拟地址分隔：
///
/// ```
/// |    9   |    9   |    9   |   12   |
/// | VPN[2] | VPN[1] | VPN[0] | offset |
///
/// // 注：虽然 VPN 直译为虚拟页码，但是它本质上是物理页的索引
/// ```
///
/// 2. 根据 VPN 查找3级内存分页，流程如下
///
/// ```
/// 1.假设虚拟地址为：
///     VA = 0x012345678
///
/// 2.计算虚拟页码和偏移：
///     VPN[2] = (VA >> 30) & 0x1FF = 0
///     VPN[1] = (VA >> 21) & 0x1FF = 0x91
///     VPN[0] = (VA >> 12) & 0x1FF = 0x145
///     offset = 0x678
///
/// 3.虚拟地址的基地址为：
///     L2_base = 0x1000_0000
///
/// 4.获取一级页表的物理地址，通过该地址获取二级页表项的物理内存地址
///     计算过程：
///         1.先通过 MMU 获取 虚拟地址 0x1000_0000 对应的物理页码 0x2001
///         2.上面我们说过虚拟页码其实是物理页页表项的索引，因此二级页表的页表项地址为：PA[0] = 0x2001 + (0x00 * 8) = 0x2001
///             注：这里的 8 是指 8字节，内存寻址按一个字节为单位，因此 8字节 刚好就是一个页表项的长度 64位
///
///     L2_index = 0 → L2_PTE_addr = 0x1000_0000 → PPN=0x2001
///
/// 5.读取二级页表项内容，计算三级页表项物理内存地址，计算过程如上：
///     计算过程：PA[1] = 0x2001 << 12 + (0x91 * 8) = 0x2001_0488
///
///     L1_base = 0x2001_0000
///     L1_index = 0x24 → L1_PTE_addr = 0x2001_0488 → PPN=0x3002
///
/// 6.读取三级页表项内容，计算需要获取的数据所在的物理内存地址，计算过程如上：
///     计算过程：PA[2] = 0x3002 << 12 + (0x145 * 8) = 0x3002_0A28
///
///     L0_base = 0x3002_0000
///     L0_index = 0x145 → L0_PTE_addr = 0x3002_0A28 → PPN=0x4003
///
/// 7.计算数据所在的物理地址
///     计算过程：PA = 0x4003 << 12 + 0x678（偏移值） = 0x4003_0678
///
/// PA = 0x4003_0000 + 0x678 = 0x4003_0678
/// ```
///

bitflags! {
    ///
    /// 页表项标志位
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/9
    pub struct PTEFlags: u8 {
        const V = 1 << 0;  // 仅当位 V 为 1 时，页表项才是合法的
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3; // R(Read)/W(Write)/X(eXecute)：分别控制索引到这个页表项的对应虚拟页面是否允许读/写/执行；
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

///
/// 页表项
///
/// @author: tryte
///
/// @date: 2026/1/9
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    ///
    /// 实例化
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/9
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }

    ///
    /// 清空
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/9
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }

    ///
    /// 转换成物理页码
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/9
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    ///
    /// 获取页表项标志位
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/9
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    ///
    /// 判断页表项是否有效
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/15
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }

    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

pub struct PageTable {
    // 根页号
    root_ppn: PhysPageNum,
    // 页帧
    frames: Vec<FrameTracker>,
}

impl PageTable {
    ///
    /// 初始化页表
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/14
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    ///
    /// 临时用于从用户空间获取参数
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/16
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    ///
    /// 查找页表项，不存在则创建
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/15
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        // 切割虚拟地址为3段，虚拟地址每段都相当于页表页的索引
        let idxs = vpn.indexes();
        // 页表根地址
        let mut ppn = self.root_ppn;
        // 空页表项
        let mut result: Option<&mut PageTableEntry> = None;
        // 循环虚拟地址分段
        for (i, idx) in idxs.iter().enumerate() {
            // 获取当前级页表的页表项来查找下一级页表项
            let pte = &mut ppn.get_pte_array()[*idx];
            // 到3级后退出，三级页表页空间已在上一次循环分配
            if i == 2 {
                result = Some(pte);
                break;
            }
            // 判断该页表项是否有效，无效代表下一级页表不存在，需要创建
            if !pte.is_valid() {
                // 创建下一级页表页
                let frame = frame_alloc().unwrap();
                // 创建页表页的页表项
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                // 记录页帧
                self.frames.push(frame);
            }
            // 切换到下一级页表的起始物理地址
            ppn = pte.ppn();
        }
        // 返回三级页表的页表项
        result
    }

    ///
    /// 查找页表项
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/15
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }

    ///
    /// 将物理页映射到虚拟内存页
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/15
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        // 创建页表项
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        // 设置三级页表项内容
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V)
    }

    ///
    /// 在多级页表中删除一个键值对
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/15
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invaild before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    ///
    /// 虚拟地址转页表项
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/16
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }

    ///
    /// 获取 mmu 设置
    ///
    /// mmu 由名为 satp 的寄存器进行设置，satp 的字段分布，含义如下：
    ///
    ///
    /// ```
    /// |63      60|59          44|43              0|
    /// |   MODE   |     ASID     |       PPN       |
    /// ```
    ///
    /// - MODE 控制 CPU 使用哪种页表实现
    ///
    ///         当 MODE 设置为 0 的时候，代表所有访存都被视为物理地址；而设置为 8 的时候，SV39 分页机制被启用，所有 S/U 特权级的访存被视为一个 39 位的虚拟地址，它们需要先经过 MMU 的地址转换流程
    ///
    /// - ASID 表示地址空间标识符
    ///
    /// - PPN 存的是根页表所在的物理页号
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/22
    pub fn token(&self) -> usize {
        // 开启 MMU，使用分页机制并设置根页表所在的物理页号
        8usize << 60 | self.root_ppn.0
    }
}
