use crate::drivers::bus::virtio::VirtioHal;
use crate::sync::UpIntrFreeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::Any;
use embedded_graphics::pixelcolor::Rgb888;
use lazy_static::lazy_static;
use tinybmp::Bmp;
use virtio_drivers::{VirtIOGpu, VirtIOHeader};

lazy_static! {
    pub static ref GPU_DEVICE: Arc<dyn GpuDevice> = Arc::new(VirtIOGpuWrapper::new());
}

/// 光标图片数据
static BMP_DATA: &[u8] = include_bytes!("../../assert/mouse.bmp");

/// GPU 外设 MMIO 地址
const VIRTIO7: usize = 0x10_007_000;

pub trait GpuDevice: Send + Sync + Any {
    ///
    /// 分配缓存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn get_framebuffer(&self) -> &mut [u8];

    ///
    /// 将数据刷入缓存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn flush(&self);
}

///
/// GPU 驱动包装器
///
/// @author: tryte
///
/// @date: 2026/6/15
pub struct VirtIOGpuWrapper {
    /// GPU 驱动
    gpu: UpIntrFreeCell<VirtIOGpu<'static, VirtioHal>>,
    /// 显示器图像内存操作地址
    fb: &'static [u8],
}

impl VirtIOGpuWrapper {
    ///
    /// 初始化驱动
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    pub fn new() -> Self {
        unsafe {
            // 创建 GPU
            let mut virtio =
                VirtIOGpu::<VirtioHal>::new(&mut *(VIRTIO7 as *mut VirtIOHeader)).unwrap();

            // 初始化显示器
            let f_buffer = virtio.setup_framebuffer().unwrap();
            let len = f_buffer.len();
            let ptr = f_buffer.as_mut_ptr();
            let fb = core::slice::from_raw_parts_mut(ptr, len);

            // 创建光标
            let bmp = Bmp::<Rgb888>::from_slice(BMP_DATA).unwrap();
            let raw = bmp.as_raw();
            let mut b = Vec::new();
            for i in raw.image_data().chunks(3) {
                let mut v = i.to_vec();
                b.append(&mut v);
                if i == [255, 255, 255] {
                    b.push(0x00);
                } else {
                    // 透明度
                    b.push(0xFF);
                }
            }
            // 设置光标
            virtio.setup_cursor(b.as_slice(), 50, 50, 50, 50).unwrap();

            Self {
                gpu: UpIntrFreeCell::new(virtio),
                fb,
            }
        }
    }
}

impl GpuDevice for VirtIOGpuWrapper {
    ///
    /// 分配缓存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn get_framebuffer(&self) -> &mut [u8] {
        unsafe {
            let ptr = self.fb.as_ptr() as *const _ as *mut u8;
            core::slice::from_raw_parts_mut(ptr, self.fb.len())
        }
    }

    ///
    /// 将数据刷入缓存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn flush(&self) {
        self.gpu.exclusive_access().flush().unwrap()
    }
}
