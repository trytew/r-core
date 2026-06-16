#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use user_lib::{close, open, read, OpenFlags};

///
/// 查看文件
///
/// @author: tryte
///
/// @date: 2026/4/8
#[unsafe(no_mangle)]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
    assert_eq!(argc, 2);

    let fd = open(argv[1], OpenFlags::RDONLY);
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
