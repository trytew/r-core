#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{close, get_time, open, write, OpenFlags};

///
/// 巨量写
///
/// @author: tryte
///
/// @date: 2026/4/8
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let mut buffer = [0_u8; 1024];
    for (i, ch) in buffer.iter_mut().enumerate() {
        *ch = i as u8;
    }
    let f = open("test\0", OpenFlags::CREATE | OpenFlags::WRONLY);
    if f < 0 {
        panic!("open test file failed!");
    }
    let f = f as usize;
    let start = get_time();
    let size_mb = 1_usize;
    for _ in 0..1024 * size_mb {
        write(f, &buffer);
    }
    close(f);
    let time_ms = (get_time() - start) as usize;
    let speed_kbs = size_mb * 1_000_000 / time_ms;
    println!(
        "{} MiB written, time cost = {}ms, write speed = {}KiB/s",
        size_mb, time_ms, speed_kbs,
    );
    0
}
