#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{close, open, read, OpenFlags};

///
/// 查看文件a
///
/// @author: tryte
///
/// @date: 2026/4/8
#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let fd = open("file_a\0", OpenFlags::RDONLY);
    if fd == -1 {
        panic!("Error occured while opening file");
    }
    let fd = fd as usize;
    let mut buf = [0_u8; 256];
    loop {
        let size = read(fd, &mut buf) as usize;
        if size == 0 {
            break;
        }
        println!("{}", core::str::from_utf8(&buf[..size]).unwrap());
    }
    close(fd);
    0
}
