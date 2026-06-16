#![no_std]
#![no_main]

extern crate user_lib;

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{DrawTarget, Point, Primitive, RgbColor, Size};
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle, Triangle};
use embedded_graphics::Drawable;
use user_lib::{Display, VIRT_GPU_X_RES, VIRT_GPU_Y_RES};

const INIT_X: i32 = 80;
const INIT_Y: i32 = 400;
const RECT_SIZE: u32 = 150;

pub struct DrawingBoard {
    display: Display,
    latest_pos: Point,
}

impl DrawingBoard {
    pub fn new() -> Self {
        Self {
            display: Display::new(Size::new(VIRT_GPU_X_RES, VIRT_GPU_Y_RES)),
            latest_pos: Point::new(INIT_X, INIT_Y),
        }
    }

    fn paint(&mut self) {
        Rectangle::with_center(self.latest_pos, Size::new(RECT_SIZE, RECT_SIZE))
            .into_styled(PrimitiveStyle::with_stroke(Rgb888::RED, 10))
            .draw(&mut self.display)
            .ok();
        Circle::new(self.latest_pos + Point::new(-70, -300), 150)
            .into_styled(PrimitiveStyle::with_fill(Rgb888::BLUE))
            .draw(&mut self.display)
            .ok();
        Triangle::new(
            self.latest_pos + Point::new(0, 150),
            self.latest_pos + Point::new(80, 120),
            self.latest_pos + Point::new(-120, 300),
        )
        .into_styled(PrimitiveStyle::with_stroke(Rgb888::GREEN, 10))
        .draw(&mut self.display)
        .ok();
    }
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let mut board = DrawingBoard::new();
    let _ = board.display.clear(Rgb888::BLACK).unwrap();
    for _ in 0..5 {
        board.latest_pos.x += RECT_SIZE as i32 + 20;
        board.paint();
    }
    board.display.flush();
    0
}
