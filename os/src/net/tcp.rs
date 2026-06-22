use crate::drivers::NET_DEVICE;
use crate::fs::File;
use crate::mm::UserBuffer;
use crate::net::socket::{add_socket, get_s_by_index, pop_data, remove_socket};
use crate::net::{net_interrupt_handler, LOCK_NET_STACK};
use alloc::vec;
use lose_net_stack::packets::tcp::TCPPacket;
use lose_net_stack::{IPv4, MacAddress, TcpFlags};

///
/// TCP 连接套接字
///
/// @author: tryte
///
/// @date: 2026/6/22
pub struct TCP {
    /// 客户端 IP
    pub target: IPv4,
    /// 监听端口号
    pub s_port: u16,
    /// 客户端端口号
    pub d_port: u16,
    #[allow(unused)]
    pub seq: u32,
    #[allow(unused)]
    pub ack: u32,
    /// 连接索引号
    pub socket_index: usize,
}

impl TCP {
    ///
    /// 创建 TCP 连接套接字
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    pub fn new(target: IPv4, s_port: u16, d_port: u16, seq: u32, ack: u32) -> Self {
        let index = add_socket(target, s_port, d_port).expect("can't add socket");

        Self {
            target,
            s_port,
            d_port,
            seq,
            ack,
            socket_index: index,
        }
    }
}

impl File for TCP {
    ///
    /// 是否可读
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    fn readable(&self) -> bool {
        true
    }

    ///
    /// 是否可写
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    fn writeable(&self) -> bool {
        true
    }

    ///
    /// 读数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    fn read(&self, mut buf: UserBuffer) -> usize {
        loop {
            if let Some(data) = pop_data(self.socket_index) {
                // 复制数据到用户空间
                let data_len = data.len();
                let mut left = 0;
                for i in 0..buf.buffers.len() {
                    let buffer_i_len = buf.buffers[i].len().min(data_len - left);

                    buf.buffers[i][..buffer_i_len]
                        .copy_from_slice(&data[left..(left + buffer_i_len)]);

                    left += buffer_i_len;
                    if left == data_len {
                        break;
                    }
                }
                return left;
            } else {
                // 读取网卡数据
                net_interrupt_handler();
            }
        }
    }

    ///
    /// 写数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    fn write(&self, buf: UserBuffer) -> usize {
        // 获取网络协议栈
        let lose_net_stack = LOCK_NET_STACK.0.exclusive_access();

        // 复制用户空间数据
        let mut data = vec![0_u8; buf.len()];

        let mut left = 0;
        for i in 0..buf.buffers.len() {
            data[left..(left + buf.buffers[i].len())].copy_from_slice(buf.buffers[i]);
            left += buf.buffers[i].len();
        }

        let len = data.len();

        // 获取 socket
        let (ack, seq) = get_s_by_index(self.socket_index).map_or((0, 0), |x| x);

        // 构建 TCP 网络数据包
        let tcp_packet = TCPPacket {
            source_ip: lose_net_stack.ip,
            source_mac: lose_net_stack.mac,
            source_port: self.s_port,
            dest_ip: self.target,
            dest_mac: MacAddress::new([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
            dest_port: self.d_port,
            data_len: len,
            seq,
            ack,
            flags: TcpFlags::A,
            win: 65535,
            urg: 0,
            data: data.as_ref(),
        };

        // 发送数据
        NET_DEVICE.transmit(&tcp_packet.build_data());
        len
    }
}

impl Drop for TCP {
    ///
    /// 析构 TCP
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    fn drop(&mut self) {
        // 从连接表删除连接
        remove_socket(self.socket_index)
    }
}
