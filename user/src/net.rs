use crate::syscall::{sys_accept, sys_connect, sys_listen};

///
/// 创建UDP
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn connect(ip: u32, s_port: u16, d_port: u16) -> isize {
    sys_connect(ip, s_port, d_port)
}

///
/// 监听端口
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn listen(s_port: u16) -> isize {
    sys_listen(s_port)
}

///
/// 建立连接
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn accept(socket_fd: usize) -> isize {
    sys_accept(socket_fd)
}
