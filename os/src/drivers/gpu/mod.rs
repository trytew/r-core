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

static BMP_DATA: &[u8] = include_bytes!("../../assert/mouse.bmp");

const VIRTIO7: usize = 0x10_007_000;

pub trait GpuDevice: Send + Sync + Any {
    fn get_framebuffer(&self) -> &mut [u8];

    fn flush(&self);
}

pub struct VirtIOGpuWrapper {
    gpu: UpIntrFreeCell<VirtIOGpu<'static, VirtioHal>>,
    fb: &'static [u8],
}

impl VirtIOGpuWrapper {
    pub fn new() -> Self {
        unsafe {
            let mut virtio =
                VirtIOGpu::<VirtioHal>::new(&mut *(VIRTIO7 as *mut VirtIOHeader)).unwrap();

            let f_buffer = virtio.setup_framebuffer().unwrap();
            let len = f_buffer.len();
            let ptr = f_buffer.as_mut_ptr();
            let fb = core::slice::from_raw_parts_mut(ptr, len);

            let bmp = Bmp::<Rgb888>::from_slice(BMP_DATA).unwrap();
            let raw = bmp.as_raw();
            let mut b = Vec::new();
            for i in raw.image_data().chunks(3) {
                let mut v = i.to_vec();
                b.append(&mut v);
                if i == [255, 255, 255] {
                    b.push(0x00);
                } else {
                    b.push(0xFF);
                }
            }
            virtio.setup_cursor(b.as_slice(), 50, 50, 50, 50).unwrap();

            Self {
                gpu: UpIntrFreeCell::new(virtio),
                fb,
            }
        }
    }
}

impl GpuDevice for VirtIOGpuWrapper {
    fn get_framebuffer(&self) -> &mut [u8] {
        unsafe {
            let ptr = self.fb.as_ptr() as *const _ as *mut u8;
            core::slice::from_raw_parts_mut(ptr, self.fb.len())
        }
    }

    fn flush(&self) {
        self.gpu.exclusive_access().flush().unwrap()
    }
}
