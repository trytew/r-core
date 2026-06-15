#![no_std]
#![no_main]

extern crate user_lib;

use embedded_graphics::prelude::Size;
use user_lib::{Display, VIRT_GPU_X_RES, VIRT_GPU_Y_RES};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let mut display = Display::new(Size::new(VIRT_GPU_X_RES, VIRT_GPU_Y_RES));
    display.paint_on_framebuffer(|fb| {
        for y in 0..VIRT_GPU_Y_RES as usize {
            for x in 0..VIRT_GPU_X_RES as usize {
                let idx = (y * VIRT_GPU_X_RES as usize + x) * 4;
                fb[idx] = x as u8;
                fb[idx + 1] = y as u8;
                fb[idx + 2] = (x + y) as u8;
            }
        }
    });
    display.flush();
    0
}
