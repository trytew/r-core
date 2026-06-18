use crate::fs::File;
use crate::mm::UserBuffer;
use crate::net::net_interrupt_handler;
use crate::net::socket::{add_socket, pop_data};
use lose_net_stack::IPv4;

pub struct TCP {
    pub target: IPv4,
    pub s_port: u16,
    pub d_port: u16,
    #[allow(unused)]
    pub seq: u32,
    #[allow(unused)]
    pub ack: u32,
    pub socket_index: usize,
}

impl TCP {
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
    fn readable(&self) -> bool {
        true
    }

    fn writeable(&self) -> bool {
        true
    }

    fn read(&self, mut buf: UserBuffer) -> usize {
        loop {
            if let Some(data) = pop_data(self.socket_index) {
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
                net_interrupt_handler();
            }
        }
    }

    fn write(&self, buf: UserBuffer) -> usize {
        todo!()
    }
}
