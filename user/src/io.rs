use crate::syscall::{sys_event_get, sys_framebuffer, sys_framebuffer_flush, sys_key_pressed};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{DrawTarget, OriginDimensions, RgbColor, Size};
use embedded_graphics::Pixel;
use virtio_input_decoder::{DecodeType, Decoder};

pub const VIRT_GPU_X_RES: u32 = 1280;
pub const VIRT_GPU_Y_RES: u32 = 800;

pub const VIRT_GPU_LEN: usize = (VIRT_GPU_X_RES * VIRT_GPU_Y_RES * 4) as usize;

///
/// 显示器
///
/// @author: tryte
///
/// @date: 2026/6/15
pub struct Display {
    /// 显示器大小
    pub size: Size,
    /// 显示器操作内存地址
    pub fb: &'static mut [u8],
}

impl Display {
    ///
    /// 初始化
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    pub fn new(size: Size) -> Self {
        // 获取 GPU 操作内存
        let fb_ptr = framebuffer() as *mut u8;
        let fb = unsafe { core::slice::from_raw_parts_mut(fb_ptr, VIRT_GPU_LEN) };
        Self { size, fb }
    }

    ///
    /// 获取操作内存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    pub fn framebuffer(&mut self) -> &mut [u8] {
        self.fb
    }

    ///
    /// 绘画显示器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    pub fn paint_on_framebuffer(&mut self, p: impl FnOnce(&mut [u8]) -> ()) {
        p(self.framebuffer())
    }

    ///
    /// 更新显示器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    pub fn flush(&self) {
        framebuffer_flush();
    }
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        self.size
    }
}

impl DrawTarget for Display {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        pixels.into_iter().for_each(|px| {
            let idx = (px.0.y * VIRT_GPU_X_RES as i32 + px.0.x) as usize * 4;
            if idx + 2 >= self.fb.len() {
                return;
            }
            self.fb[idx] = px.1.b();
            self.fb[idx + 1] = px.1.g();
            self.fb[idx + 2] = px.1.r();
        });
        Ok(())
    }
}

#[repr(C)]
pub struct InputEvent {
    pub event_type: u16,
    pub code: u16,
    pub value: u32,
}

impl InputEvent {
    pub fn decode(&self) -> Option<DecodeType> {
        Decoder::decode(
            self.event_type as usize,
            self.code as usize,
            self.value as usize,
        )
        .ok()
    }
}

impl From<u64> for InputEvent {
    fn from(mut v: u64) -> Self {
        let value = v as u32;
        v >>= 32;
        let code = v as u16;
        v >>= 16;
        let event_type = v as u16;
        Self {
            event_type,
            code,
            value,
        }
    }
}

///
/// 分配缓存页
///
/// @author: tryte
///
/// @date: 2026/6/15
pub fn framebuffer() -> isize {
    sys_framebuffer()
}

///
/// 将数据刷入缓存页
///
/// @author: tryte
///
/// @date: 2026/6/15
pub fn framebuffer_flush() -> isize {
    sys_framebuffer_flush()
}

///
/// 获取中断事件
///
/// @author: tryte
///
/// @date: 2026/6/11
pub fn event_get() -> Option<InputEvent> {
    let raw_value = sys_event_get();
    if raw_value == 0 {
        None
    } else {
        Some((raw_value as u64).into())
    }
}

///
/// 键盘是否按下
///
/// @author: tryte
///
/// @date: 2026/6/16
pub fn key_pressed() -> bool {
    if sys_key_pressed() == 1 { true } else { false }
}
