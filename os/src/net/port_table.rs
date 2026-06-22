use crate::fs::File;
use crate::mm::UserBuffer;
use crate::net::tcp::TCP;
use crate::sync::UpIntrFreeCell;
use crate::task::TaskControlBlock;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use lose_net_stack::packets::tcp::TCPPacket;

lazy_static! {
    /// 端口监听表
    static ref LISTEN_TABLE: UpIntrFreeCell<Vec<Option<Port>>> =
        unsafe { UpIntrFreeCell::new(Vec::new()) };
}

///
/// 端口
///
/// @author: tryte
///
/// @date: 2026/6/22
pub struct Port {
    /// 端口号
    pub port: u16,
    /// 是否有连接可接受
    pub receivable: bool,
    /// 调度线程
    pub schedule: Option<Arc<TaskControlBlock>>,
}

///
/// 端口套接字
///
/// @author: tryte
///
/// @date: 2026/6/22
pub struct PortFd(usize);

impl PortFd {
    ///
    /// 实例化端口套接字
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    pub fn new(port_index: usize) -> Self {
        // 记录端口监听表索引
        PortFd(port_index)
    }
}

impl Drop for PortFd {
    fn drop(&mut self) {
        LISTEN_TABLE.exclusive_access()[self.0] = None
    }
}

impl File for PortFd {
    fn readable(&self) -> bool {
        false
    }

    fn writeable(&self) -> bool {
        false
    }

    fn read(&self, _buf: UserBuffer) -> usize {
        0
    }

    fn write(&self, _buf: UserBuffer) -> usize {
        0
    }
}

///
/// 监听端口
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn listen(port: u16) -> Option<usize> {
    let mut listen_table = LISTEN_TABLE.exclusive_access();
    let mut index = usize::MAX;
    for i in 0..listen_table.len() {
        if listen_table[i].is_none() {
            index = i;
            break;
        }
    }

    // 创建端口监听
    let listen_port = Port {
        port,
        receivable: false,
        schedule: None,
    };

    // 如果端口监听表没有空位置则添加
    if index == usize::MAX {
        listen_table.push(Some(listen_port));
        Some(listen_table.len() - 1)
    } else {
        listen_table[index] = Some(listen_port);
        Some(index)
    }
}

///
/// 接受连接
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn accept(listen_index: usize, task: Arc<TaskControlBlock>) {
    let mut listen_table = LISTEN_TABLE.exclusive_access();
    assert!(listen_index < listen_table.len());
    // 获取监听端口
    let listen_port = listen_table[listen_index].as_mut();
    assert!(listen_port.is_some());
    let listen_port = listen_port.unwrap();
    // 设置为可接受
    listen_port.receivable = true;
    // 记录线程
    listen_port.schedule = Some(task);
}

///
/// 查看监听是否可建立连接
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn port_acceptable(listen_index: usize) -> bool {
    let mut listen_table = LISTEN_TABLE.exclusive_access();
    assert!(listen_index < listen_table.len());

    let listen_port = listen_table[listen_index].as_mut();
    listen_port.map_or(false, |x| x.receivable)
}

///
/// 建立连接
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn accept_connection(_port: u16, tcp_packet: &TCPPacket, task: Arc<TaskControlBlock>) {
    // 获取线程
    let process = task.process.upgrade().unwrap();
    let mut inner = process.inner_exclusive_access();
    // 分配连接套接字
    let fd = inner.alloc_fd();

    // 构建 TCP 连接套接字
    let tcp_socket = TCP::new(
        tcp_packet.source_ip,
        tcp_packet.dest_port,
        tcp_packet.source_port,
        tcp_packet.seq,
        tcp_packet.ack,
    );

    // 记录到线程的套接字列表中
    inner.fd_table[fd] = Some(Arc::new(tcp_socket));

    // 返回线程套接字索引号
    let cx = task.inner_exclusive_access().get_trap_cx();
    cx.x[10] = fd;
}

///
/// 检查并建立连接
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn check_accept(port: u16, tcp_packet: &TCPPacket) -> Option<()> {
    LISTEN_TABLE.exclusive_session(|listen_table| {
        // 查找端口是否正在监听
        let mut listen_ports: Vec<&mut Option<Port>> = listen_table
            .iter_mut()
            .filter(|x| match x {
                None => false,
                Some(t) => t.port == port && t.receivable == true,
            })
            .collect();

        if listen_ports.len() == 0 {
            // 无端口监听
            None
        } else {
            let listen_port = listen_ports[0].as_mut().unwrap();
            let task = listen_port.schedule.clone().unwrap();
            listen_port.schedule = None;
            listen_port.receivable = false;

            // 建立连接
            accept_connection(port, tcp_packet, task);
            Some(())
        }
    })
}
