use crate::drivers::GPU_DEVICE;
use crate::mm::{MapArea, MapPermission, MapType, PhysAddr, VirtAddr};
use crate::task::current_process;

const FB_VADDR: usize = 0x10_000_000;

///
/// 分配缓存页
///
/// @author: tryte
///
/// @date: 2026/6/15
pub fn sys_framebuffer() -> isize {
    // 获取显示器图像操作内存地址
    let fb = GPU_DEVICE.get_framebuffer();
    let len = fb.len();
    let fb_start_pa = PhysAddr::from(fb.as_ptr() as usize);
    assert!(fb_start_pa.aligned());
    let fb_start_ppn = fb_start_pa.floor();
    let fb_start_vpn = VirtAddr::from(FB_VADDR).floor();
    let pn_offset = fb_start_ppn.0 as isize - fb_start_vpn.0 as isize;

    // 将显示器图像的操作地址映射进进程内存空间
    let current_process = current_process();
    let mut inner = current_process.inner_exclusive_access();
    inner.memory_set.push(
        MapArea::new(
            FB_VADDR.into(),
            (FB_VADDR + len).into(),
            MapType::Linear(pn_offset),
            MapPermission::R | MapPermission::W | MapPermission::U,
        ),
        None,
    );
    FB_VADDR as isize
}

///
/// 刷新显示器
///
/// @author: tryte
///
/// @date: 2026/6/15
pub fn sys_framebuffer_flush() -> isize {
    GPU_DEVICE.flush();
    0
}
