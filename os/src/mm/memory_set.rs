use super::{PTEFlags, PhysPageNum, VPNRange, VirtPageNum};
use crate::boards::MEMORY_END;
use crate::config::{MMIO, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE};
use crate::mm::PageTable;
use crate::mm::address::{PhysAddr, StepByOne, VirtAddr};
use crate::mm::frame_allocator::{FrameTracker, frame_alloc};
use crate::mm::page_table::PageTableEntry;
use crate::println;
use crate::sync::UpSafeCell;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;
use core::arch::asm;
use lazy_static::lazy_static;
use riscv::register::satp;

lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UpSafeCell<MemorySet>> =
        Arc::new(unsafe { UpSafeCell::new(MemorySet::new_kernel()) });
}

bitflags! {
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

///
/// 内存地址映射类型
///
/// @author: tryte
///
/// @date: 2026/1/21
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical, // 恒等映射
    Framed,    // 每个虚拟页面都有一个新分配的物理页帧与之对应
}

///
/// 内存区域描述
///
/// @author: tryte
///
/// @date: 2026/1/21
pub struct MapArea {
    vpn_range: VPNRange,                              // 虚拟内存区间
    data_frames: BTreeMap<VirtPageNum, FrameTracker>, // 虚拟内存页号和物理内存页号映射
    map_type: MapType,                                // 内存页类型
    map_perm: MapPermission,                          // 内存页权限
}

impl MapArea {
    ///
    /// 创建内存区域描述
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/21
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        // 获取起始虚拟页号
        let start_vpn: VirtPageNum = start_va.floor();
        // 获取结束虚拟页号
        let end_vpn: VirtPageNum = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    ///
    /// 根据虚拟内存页新建物理内存页并加入到内存区域描述
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/24
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        // 判断虚拟内存地址类型分配堆空间并获取物理内存地址
        match self.map_type {
            MapType::Identical => {
                // 恒等映射，虚拟内存地址 = 物理内存地址
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                // 分级页表
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }
        // 添加页表项标志位
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        // 将物理页映射到虚拟内存页
        page_table.map(vpn, ppn, pte_flags);
    }

    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }

    ///
    /// 批量映射多个物理页到虚拟地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/24
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    #[allow(unused)]
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }

    #[allow(unused)]
    pub fn shrink_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
        for vpn in VPNRange::new(new_end, self.vpn_range.get_end()) {
            self.map_one(page_table, vpn);
        }
    }

    #[allow(unused)]
    pub fn append_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
        for vpn in VPNRange::new(self.vpn_range.get_end(), new_end) {
            self.map_one(page_table, vpn);
        }
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }

    ///
    /// 复制数据到内存地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/29
    pub fn copy_data(&mut self, page_table: &PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        // 获取内存区域的起始页
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            // 将数据整页切割
            let src = &data[start..len.min(start + PAGE_SIZE)];
            // 查找虚拟内存页对应的物理内存页
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            // 将数据拷贝到指定位置
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            // 获取下一页
            current_vpn.step();
        }
    }
}

unsafe extern "C" {
    fn stext(); // text 段起始位置
    fn etext(); // text 段结束位置
    fn srodata(); // 只读段起始位置
    fn erodata(); // 只读段结束位置
    fn sdata(); // 常量数据段起始位置
    fn edata(); // 常量数据段结束位置
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

///
/// 内存区域描述集合
///
/// @author: tryte
///
/// @date: 2026/1/21
pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    ///
    /// 创建内存区域描述集合
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/21
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    ///
    /// 将内容复制到指定内存区域，并添加到内存区域描述集合
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/24
    pub fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&self.page_table, data);
        }
        self.areas.push(map_area);
    }

    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    ///
    /// 给跳板映射虚拟地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/21
    fn map_trampoline(&mut self) {
        // 将 trap.asm 的汇编代码映射到最高虚拟内存页，因为在 linker.ld 中使用了对齐语句，因此 strampoline 所在地址一定在物理页的起始位置
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as *const () as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    ///
    /// 将内核代码和内核堆内存映射以恒等映射方式到虚拟内存地址
    ///
    /// 此时物理内存如下：
    ///
    ///
    ///
    /// ```
    /// 低地址
    /// |                                           |
    /// |                                           |
    /// |-------------------------------------------|--> 内核空间起始
    /// | .text   内核代码                            |
    /// |   |                                       |
    /// |   |--trampoline                           |
    /// |                                           |
    /// | .rodata 常量                               |
    /// | .data   已初始化全局变量                     |
    /// | .bss    零初始化全局变量                     |
    /// |   |                                       |
    /// |   |-- HEAP_SPACE [3MB]（内核内存分配区）      |
    /// |   |    |                                  |
    /// |   |    |  (KERNEL_SPACE.areas)            |
    /// |   |    |--Vec<MapArea> 的元素              |
    /// |   |    |   |                              |
    /// |   |    |   |--MapArea                     |
    /// |   |    |       |                          |
    /// |   |    |       |--vpn_range               |
    /// |   |    |       |                          |
    /// |   |    |       |--data_frames (BTreeMap)  |
    /// |   |    |       |   |                      |
    /// |   |    |       |   |-- BTreeMap Item      |
    /// |   |    |       |                          |
    /// |   |    |       |--map_type                |
    /// |   |    |       |                          |
    /// |   |    |       |--map_perm                |
    /// |   |    |                                  |
    /// |   |    |  (KERNEL_SPACE.PageTable.frames) |
    /// |   |    |--Vec<FrameTracker> 的元素         |
    /// |   |    |   |                              |
    /// |   |    |   |--FrameTracker                |
    /// |   |                                       |
    /// |   |-- FRAME_ALLOCATOR                     |
    /// |   |                                       |
    /// |   |-- KERNEL_SPACE                        |
    /// |        |                                  |
    /// |        |-- PageTable                      |
    /// |        |    |                             |
    /// |        |    |-- root_ppn                  |
    /// |        |    |                             |
    /// |        |    |-- frames (Vec)              |
    /// |        |                                  |
    /// |        |-- areas (Vec<MapArea>)           |
    /// |                                           |
    /// | ekernel                                   |
    /// |-------------------------------------------|--> 内核堆起始
    /// | | 一级页表页（4KB）                          |
    /// | ----------------------------------------  |
    /// | | 内存页（4KB）                             |
    /// | ----------------------------------------  |
    /// | | 内存页（4KB）                             |
    /// | ----------------------------------------  |
    /// | | ...                                     |
    /// |                                           |
    /// |（内核空间和内核堆一共占用 128MB 物理内存）       |
    /// | MEMORY_END                                |    内核堆结束/
    /// |-------------------------------------------|--> 内核空间结束
    /// |                                           |
    /// 高地址
    /// ```
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/22
    pub fn new_kernel() -> Self {
        // 内存区域集合
        let mut memory_set = Self::new_bare();

        // 映射跳板所在的内存页
        memory_set.map_trampoline();

        // 以恒等映射方式将内核各段代码映射到内存区域描述
        println!(
            ".text [{:#x}, {:#x})",
            stext as *const () as usize, etext as *const () as usize,
        );
        println!("mapping .text section");
        memory_set.push(
            MapArea::new(
                (stext as *const () as usize).into(),
                (etext as *const () as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );

        println!(
            ".rodata [{:#x}, {:#x})",
            srodata as *const () as usize, erodata as *const () as usize,
        );
        println!("mapping .rodata section");
        memory_set.push(
            MapArea::new(
                (srodata as *const () as usize).into(),
                (erodata as *const () as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );

        println!(
            ".data [{:#x}, {:#x})",
            sdata as *const () as usize, edata as *const () as usize,
        );
        println!("mapping .data section");
        memory_set.push(
            MapArea::new(
                (sdata as *const () as usize).into(),
                (edata as *const () as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        println!(
            ".bss [{:#x}, {:#x})",
            sbss_with_stack as *const () as usize, ebss as *const () as usize,
        );
        println!("mapping .bss section");
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as *const () as usize).into(),
                (ebss as *const () as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        // 将内核使用的堆空间映射以恒等映射方式到虚拟地址
        println!("mapping physical memory");
        memory_set.push(
            MapArea::new(
                (ekernel as *const () as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        // 将 MMIO 以恒等映射方式映射到虚拟内存
        // Memory-Mapped I/O 是一种将硬件设备寄存器映射到 CPU 的内存地址空间 的方式
        println!("mapping memory-mapped registers");
        for pair in MMIO {
            memory_set.push(
                MapArea::new(
                    (*pair).0.into(),
                    ((*pair).0 + (*pair).1).into(),
                    MapType::Identical,
                    MapPermission::R | MapPermission::W,
                ),
                None,
            );
        }
        memory_set
    }

    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = Self::new_bare();
        memory_set.map_trampoline();

        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();

        // 灰页
        user_stack_bottom += PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        memory_set.push(
            MapArea::new(
                user_stack_top.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        (
            memory_set,
            user_stack_top,
            elf_header.pt2.entry_point() as usize,
        )
    }

    ///
    /// 打开 MMU
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/21
    pub fn activate(&self) {
        // 当前页表根目录是 self.page_table
        let satp = self.page_table.token();
        unsafe {
            // 从现在起 所有虚拟地址访问都走新的页表映射
            satp::write(satp);
            // 清理缓存，确保访问正确
            asm!("sfence.vma");
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    #[allow(unused)]
    pub fn shrink_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {
            area.shrink_to(&mut self.page_table, new_end.ceil());
            true
        } else {
            false
        }
    }

    #[allow(unused)]
    pub fn append_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {
            area.append_to(&mut self.page_table, new_end.ceil());
            true
        } else {
            false
        }
    }
}

#[allow(unused)]
pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.exclusive_access();
    let mid_text: VirtAddr =
        ((stext as *const () as usize + etext as *const () as usize) / 2).into();
    let mid_rodata: VirtAddr =
        ((srodata as *const () as usize + erodata as *const () as usize) / 2).into();
    let mid_data: VirtAddr =
        ((sdata as *const () as usize + edata as *const () as usize) / 2).into();

    assert!(
        !kernel_space
            .page_table
            .translate(mid_text.floor())
            .unwrap()
            .writable(),
    );

    assert!(
        !kernel_space
            .page_table
            .translate(mid_rodata.floor())
            .unwrap()
            .writable(),
    );

    assert!(
        !kernel_space
            .page_table
            .translate(mid_data.floor())
            .unwrap()
            .executable(),
    );

    println!("remap_test passed!");
}
