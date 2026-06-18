use crate::sync::UpIntrFreeCell;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use lose_net_stack::IPv4;

lazy_static! {
    static ref SOCKET_TABLE: UpIntrFreeCell<Vec<Option<Socket>>> =
        unsafe { UpIntrFreeCell::new(Vec::new()) };
}

pub struct Socket {
    pub r_addr: IPv4,
    pub l_port: u16,
    pub r_port: u16,
    pub buffers: VecDeque<Vec<u8>>,
    pub seq: u32,
    pub ack: u32,
}

pub fn set_s_a_by_index(index: usize, seq: u32, ack: u32) {
    let mut socket_table = SOCKET_TABLE.exclusive_access();

    assert!(socket_table.len() > index);
    assert!(socket_table[index].is_some());

    let sock = socket_table[index].as_mut().unwrap();

    sock.ack = ack;
    sock.seq = seq;
}

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

pub fn add_socket(r_addr: IPv4, l_port: u16, r_port: u16) -> Option<usize> {
    if get_socket(r_addr, l_port, r_port).is_some() {
        return None;
    }

    let mut socket_table = SOCKET_TABLE.exclusive_access();
    let mut index = usize::MAX;
    for i in 0..socket_table.len() {
        if socket_table[i].is_none() {
            index = i;
            break;
        }
    }

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

pub fn pop_data(index: usize) -> Option<Vec<u8>> {
    let mut socket_table = SOCKET_TABLE.exclusive_access();

    assert!(socket_table.len() > index);
    assert!(socket_table[index].is_some());

    socket_table[index].as_mut().unwrap().buffers.pop_front()
}
