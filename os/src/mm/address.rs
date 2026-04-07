use crate::config::PAGE_SIZE;
use crate::config::PAGE_SIZE_BITS;
use crate::mm::page_table::PageTableEntry;
use core::fmt::Formatter;
use core::fmt::{Debug, Write};

///
/// 当开启 MMU 后，所有内存地址访问都由直接物理内存访问变为虚拟内存访问，
/// 路径是指令访问虚拟地址后由 MMU 转换成物理地址再由 satp 这个特殊 CSR 来访问物理地址
/// 其中 MMU 的使能也是由 satp 来控制，satp 的字段分布如下：
///
/// ```
/// 63        60|59       44|43        0
///
/// |MODE (WARL)|ASID (WARL)|PPN (WARL)|
/// ```
///
/// - MODE 控制 CPU 使用哪种页表实现；当为 0 时代表直接访问物理地址
///
/// - ASID 表示地址空间标识符，这里还没有涉及到进程的概念，我们不需要管这个地方；
///
/// - PPN（Physical Page Number）存的是根页表所在的物理页号。这样，给定一个虚拟页号，CPU 就可以从三级页表的根页表开始一步步的将其映射到一个物理页号。
///
///
/// 由上面可以看出物理页号是有 44 位的，再加上页内地址偏移 12 位（512 byte）一共是 56 位，但是虚拟内存的页号只支持到 39 位，因此高地址多出 5 位是预留的
/// 物理内存地址的页内偏移和虚拟内存的页内偏移是一样的，因为采用内存分页管理它们的大小都是 4Kb(bit)
///

/// 物理地址长度
const PA_WIDTH_SV39: usize = 56;

/// 物理页号长度
const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;

/// 虚拟内存地址长度
const VA_WIDTH_SV39: usize = 39;

/// 虚拟页号（Virtual Page Number）长度
#[allow(unused)]
const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - PAGE_SIZE_BITS;

pub trait StepByOne {
    fn step(&mut self);
}

/// 物理内存页号
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);

/// 虚拟内存地址
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

/// 虚拟内存页号
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtPageNum(pub usize);

/// 物理内存地址
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
    ///
    /// 向上取整
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/8
    pub fn ceil(&self) -> PhysPageNum {
        if self.0 == 0 {
            PhysPageNum(0)
        } else {
            PhysPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
        }
    }

    ///
    /// 向下取整
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/8
    pub fn floor(&self) -> PhysPageNum {
        PhysPageNum(self.0 / PAGE_SIZE)
    }

    ///
    /// 获取页偏移
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/8
    pub fn page_offset(&self) -> usize {
        // 页偏移 = 4095 & 已使用的空间
        self.0 & (PAGE_SIZE - 1)
    }

    ///
    /// 判断是否对齐
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/8
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }

    pub fn get_ref<T>(&self) -> &'static T {
        unsafe { (self.0 as *const T).as_ref().unwrap() }
    }

    ///
    /// 获取可变指针
    ///
    /// @author: tryte
    ///
    /// @date: 2026/3/7
    pub fn get_mut<T>(&self) -> &'static mut T {
        unsafe { (self.0 as *mut T).as_mut().unwrap() }
    }
}

impl Debug for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("PA:{:#x}", self.0))
    }
}

impl From<usize> for PhysAddr {
    fn from(value: usize) -> Self {
        Self(value & ((1 << PA_WIDTH_SV39) - 1))
    }
}

impl From<PhysPageNum> for PhysAddr {
    fn from(value: PhysPageNum) -> Self {
        Self(value.0 << PAGE_SIZE_BITS)
    }
}

impl PhysPageNum {
    ///
    /// 获取页表页的所有页表项
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/9
    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = (*self).into();
        // 因为一个页表项是8字节=64位，因此一个 usize 刚好就是一个页表项，作为数组取512个元素就是一个页表项
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512) }
    }

    ///
    /// 获取页表内容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/12
    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096) }
    }

    ///
    /// 获取可变引用
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/16
    pub fn get_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = self.clone().into();
        unsafe { (pa.0 as *mut T).as_mut().unwrap() }
    }
}

impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("PPN:{:#x}", self.0))
    }
}

impl From<usize> for PhysPageNum {
    fn from(value: usize) -> Self {
        Self(value & ((1 << PPN_WIDTH_SV39) - 1))
    }
}

impl From<PhysAddr> for PhysPageNum {
    fn from(value: PhysAddr) -> Self {
        assert_eq!(value.page_offset(), 0);
        value.floor()
    }
}

impl StepByOne for PhysPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}

impl VirtAddr {
    ///
    /// 向上取整
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/15
    pub fn ceil(&self) -> VirtPageNum {
        if self.0 == 0 {
            VirtPageNum(0)
        } else {
            // 页号从0开始编号，-1是为了防止等于临界值时多分了一页空页，如：self.0=4096
            VirtPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
        }
    }

    ///
    /// 向下取整
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/15
    pub fn floor(&self) -> VirtPageNum {
        VirtPageNum(self.0 / PAGE_SIZE)
    }

    ///
    /// 计算页偏移
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/15
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }

    ///
    /// 检查页是否对齐
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/15
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("VA:{:#x}", self.0))
    }
}

impl From<VirtPageNum> for VirtAddr {
    fn from(value: VirtPageNum) -> Self {
        Self(value.0 << PAGE_SIZE_BITS)
    }
}

impl From<usize> for VirtAddr {
    ///
    /// 将 usize 转换成虚拟地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/22
    fn from(value: usize) -> Self {
        Self(value & ((1 << VA_WIDTH_SV39) - 1))
    }
}

impl VirtPageNum {
    ///
    /// 分隔虚拟地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/15
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        // 翻转顺序返回虚拟地址段内容，一级页表索引在第一位
        for i in (0..3).rev() {
            idx[i] = vpn & 511;
            vpn >>= 9;
        }
        idx
    }
}

impl From<VirtAddr> for VirtPageNum {
    fn from(value: VirtAddr) -> Self {
        assert_eq!(value.page_offset(), 0);
        value.floor()
    }
}

impl From<usize> for VirtPageNum {
    ///
    /// 将 usize 转换成虚拟地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/22
    fn from(value: usize) -> Self {
        Self(value & ((1 << VA_WIDTH_SV39) - 1))
    }
}

impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("VA:{:#x}", self.0))
    }
}

impl StepByOne for VirtPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}

impl From<PhysAddr> for usize {
    fn from(value: PhysAddr) -> Self {
        value.0
    }
}

impl From<PhysPageNum> for usize {
    fn from(v: PhysPageNum) -> Self {
        v.0
    }
}

impl From<VirtAddr> for usize {
    fn from(value: VirtAddr) -> Self {
        if value.0 >= ((1 << VA_WIDTH_SV39) - 1) {
            value.0 | (!((1 << VA_WIDTH_SV39) - 1))
        } else {
            value.0
        }
    }
}

impl From<VirtPageNum> for usize {
    fn from(value: VirtPageNum) -> Self {
        value.0
    }
}

pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}

impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        Self { current: l, end: r }
    }
}

impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.step();
            Some(t)
        }
    }
}

#[derive(Copy, Clone)]
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    l: T,
    r: T,
}

impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start: {:?} > end {:?}!", start, end);
        Self { l: start, r: end }
    }

    pub fn get_start(&self) -> T {
        self.l
    }

    pub fn get_end(&self) -> T {
        self.r
    }
}

impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    type IntoIter = SimpleRangeIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.l, self.r)
    }
}

///
/// 虚拟地址区间
///
/// @author: tryte
///
/// @date: 2026/1/21
pub type VPNRange = SimpleRange<VirtPageNum>;
