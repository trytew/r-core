use crate::sync::UpIntrFreeCell;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use lose_net_stack::IPv4;

lazy_static! {
    /// 连接表
    static ref SOCKET_TABLE: UpIntrFreeCell<Vec<Option<Socket>>> =
        unsafe { UpIntrFreeCell::new(Vec::new()) };
}

///
/// 连接
///
/// @author: tryte
///
/// @date: 2026/6/22
pub struct Socket {
    /// 对端 IP
    pub r_addr: IPv4,
    /// 本机监听端口号
    pub l_port: u16,
    /// 对端端口号
    pub r_port: u16,
    /// 数据 buffer 队列
    pub buffers: VecDeque<Vec<u8>>,
    /// 序列号（已传输字节数）
    pub seq: u32,
    /// 确认号，对端已收到字节数
    pub ack: u32,
}

///
/// 保存连接的序列号和确认号
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn set_s_a_by_index(index: usize, seq: u32, ack: u32) {
    let mut socket_table = SOCKET_TABLE.exclusive_access();

    assert!(socket_table.len() > index);
    assert!(socket_table[index].is_some());

    let sock = socket_table[index].as_mut().unwrap();

    sock.ack = ack;
    sock.seq = seq;
}

pub fn get_s_by_index(index: usize) -> Option<(u32, u32)> {
    let socket_table = SOCKET_TABLE.exclusive_access();

    assert!(index < socket_table.len());

    socket_table.get(index).map_or(None, |x| match x {
        Some(x) => Some((x.seq, x.ack)),
        None => None,
    })
}

///
/// 获取连接
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn get_socket(r_addr: IPv4, l_port: u16, r_port: u16) -> Option<usize> {
    let socket_table = SOCKET_TABLE.exclusive_access();
    for i in 0..socket_table.len() {
        let sock = &socket_table[i];
        if sock.is_none() {
            continue;
        }

        let sock = sock.as_ref().unwrap();
        if sock.r_addr == r_addr && sock.l_port == l_port && sock.r_port == r_port {
            return Some(i);
        }
    }
    None
}

///
/// 记录连接
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn add_socket(r_addr: IPv4, l_port: u16, r_port: u16) -> Option<usize> {
    // 查看连接是否已存在
    if get_socket(r_addr, l_port, r_port).is_some() {
        return None;
    }

    // 将连接加入连接记录表
    let mut socket_table = SOCKET_TABLE.exclusive_access();
    let mut index = usize::MAX;
    for i in 0..socket_table.len() {
        if socket_table[i].is_none() {
            index = i;
            break;
        }
    }

    // 构造 socket
    let socket = Socket {
        r_addr,
        l_port,
        r_port,
        buffers: VecDeque::new(),
        seq: 0,
        ack: 0,
    };

    if index == usize::MAX {
        socket_table.push(Some(socket));
        Some(socket_table.len() - 1)
    } else {
        socket_table[index] = Some(socket);
        Some(index)
    }
}

///
/// 删除连接
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn remove_socket(index: usize) {
    let mut socket_table = SOCKET_TABLE.exclusive_access();

    assert!(socket_table.len() > index);

    socket_table[index] = None;
}

///
/// 记录接收的数据
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn push_data(index: usize, data: Vec<u8>) {
    let mut socket_table = SOCKET_TABLE.exclusive_access();

    assert!(socket_table.len() > index);
    assert!(socket_table[index].is_some());

    socket_table[index]
        .as_mut()
        .unwrap()
        .buffers
        .push_back(data);
}

///
/// 弹出接收的数据
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn pop_data(index: usize) -> Option<Vec<u8>> {
    let mut socket_table = SOCKET_TABLE.exclusive_access();

    assert!(socket_table.len() > index);
    assert!(socket_table[index].is_some());

    socket_table[index].as_mut().unwrap().buffers.pop_front()
}
