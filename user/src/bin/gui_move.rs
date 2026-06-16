#![no_std]
#![no_main]

extern crate user_lib;

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{DrawTarget, Point, Primitive, RgbColor, Size};
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::Drawable;
use user_lib::console::getchar;
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
            .into_styled(PrimitiveStyle::with_stroke(Rgb888::WHITE, 1))
            .draw(&mut self.display)
            .ok();
    }

    fn unpaint(&mut self) {
        Rectangle::with_center(self.latest_pos, Size::new(RECT_SIZE, RECT_SIZE))
            .into_styled(PrimitiveStyle::with_stroke(Rgb888::BLACK, 1))
            .draw(&mut self.display)
            .ok();
    }

    pub fn move_react(&mut self, dx: i32, dy: i32) {
        let new_x = self.latest_pos.x + dx;
        let new_y = self.latest_pos.y + dy;
        let r = (RECT_SIZE / 2) as i32;
        if new_x > r
            && new_x + r < (VIRT_GPU_X_RES as i32)
            && new_y > r
            && new_y + r < (VIRT_GPU_Y_RES as i32)
        {
            self.unpaint();
            self.latest_pos.x = new_x;
            self.latest_pos.y = new_y;
            self.paint();
        }
    }
}

const LF: u8 = 0x0A_u8;
const CR: u8 = 0x0D_u8;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let mut board = DrawingBoard::new();
    let _ = board.display.clear(Rgb888::BLACK).unwrap();
    board.paint();
    board.display.flush();
    loop {
        let c = getchar();
        if c == LF || c == CR {
            break;
        }
        let mut moved = true;
        match c {
            b'w' => board.move_react(0, -10),
            b'a' => board.move_react(-10, 0),
            b's' => board.move_react(0, 10),
            b'd' => board.move_react(10, 0),
            _ => moved = false,
        }
        if moved {
            board.display.flush();
        }
    }
    0
}
