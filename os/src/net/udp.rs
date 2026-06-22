use crate::drivers::NET_DEVICE;
use crate::fs::File;
use crate::mm::UserBuffer;
use crate::net::socket::{add_socket, pop_data, remove_socket};
use crate::net::{net_interrupt_handler, LOCK_NET_STACK};
use alloc::vec;
use lose_net_stack::packets::udp::UDPPacket;
use lose_net_stack::{IPv4, MacAddress};

pub struct UDP {
    pub target: IPv4,
    pub s_port: u16,
    pub d_port: u16,
    pub socket_index: usize,
}

impl UDP {
    ///
    /// 创建UDP
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    pub fn new(target: IPv4, s_port: u16, d_port: u16) -> Self {
        let index = add_socket(target, s_port, d_port).expect("can't add socket");

        Self {
            target,
            s_port,
            d_port,
            socket_index: index,
        }
    }
}

impl File for UDP {
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
                let data_len = data.len();
                let mut left = 0;
                for i in 0..buf.buffers.len() {
                    let buffer_i_len = buf.buffers[i].len().min(data_len - left);

                    buf.buffers[i][..buffer_i_len].copy_from_slice(&data[left..buffer_i_len]);

                    left += buffer_i_len;
                    if left == data_len {
                        break;
                    }
                }
                return left;
            } else {
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
        let lose_net_stack = LOCK_NET_STACK.0.exclusive_access();

        let mut data = vec![0_u8; buf.len()];

        let mut left = 0;
        for i in 0..buf.buffers.len() {
            data[left..(left + buf.buffers[i].len())].copy_from_slice(buf.buffers[i]);
            left += buf.buffers[i].len();
        }

        let len = data.len();

        let udp_packet = UDPPacket::new(
            lose_net_stack.ip,
            lose_net_stack.mac,
            self.s_port,
            self.target,
            MacAddress::new([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
            self.d_port,
            len,
            data.as_ref(),
        );
        NET_DEVICE.transmit(&udp_packet.build_data());
        len
    }
}

impl Drop for UDP {
    fn drop(&mut self) {
        remove_socket(self.socket_index)
    }
}
