// pub fn sys_connect(r_addr:u32, l_port:u16, r_port:u16) ->isize {
//     let process = current_process();
//     let mut inner = process.inner_exclusive_access();
//     let fd = inner.alloc_fd();
//     let udp_node = UDP()
// }

use crate::net::{accept, listen, net_interrupt_handler, port_acceptable, PortFd};
use crate::println;
use crate::task::{current_process, current_task, current_trap_cx};
use alloc::sync::Arc;

///
/// 监听端口
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn sys_listen(port: u16) -> isize {
    match listen(port) {
        Some(port_index) => {
            let process = current_process();
            let mut inner = process.inner_exclusive_access();
            let fd = inner.alloc_fd();
            // 将端口套接字添加到线程
            let port_fd = PortFd::new(port_index);
            inner.fd_table[fd] = Some(Arc::new(port_fd));

            // 返回端口监听表索引
            port_index as isize
        }
        None => -1,
    }
}

///
/// 接受连接
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn sys_accept(port_index: usize) -> isize {
    println!("accepting port {}", port_index);

    let task = current_task().unwrap();
    // 接受连接
    accept(port_index, task);

    loop {
        // 处理网络外设中断
        net_interrupt_handler();
        // 查看监听端口是否可建立连接
        if !port_acceptable(port_index) {
            break;
        }
    }

    let cx = current_trap_cx();
    cx.x[10] as isize
}
