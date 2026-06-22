#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::string::String;
use user_lib::{connect, read, write};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("udp test open!");

    // 连接 10.0.2.2:26099，即宿主机的 26099 端口，2000是当前 udp 连接的数据接收地址
    let udp_fd = connect(10 << 24 | 0 << 16 | 2 << 8 | 2, 2000, 26099);

    if udp_fd < 0 {
        println!("failed to create udp connection.");
        return -1;
    }

    let buf = "Hello rCoreOs user program!";

    println!("send <{}>", buf);

    write(udp_fd as usize, buf.as_bytes());

    println!("udp send done, waiting for reply.");

    let mut buf = vec![0_u8; 1024];

    let len = read(udp_fd as usize, &mut buf);

    if len < 0 {
        println!("can't receive udp packet");
        return -1;
    }

    let recv_str = String::from_utf8_lossy(&buf[..len as usize]);

    println!("receive reply {}", recv_str);

    0
}
