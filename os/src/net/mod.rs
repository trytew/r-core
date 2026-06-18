use crate::drivers::NET_DEVICE;
use crate::net::port_table::check_accept;
use crate::net::socket::{get_socket, push_data, set_s_a_by_index};
use crate::sync::UpIntrFreeCell;
use alloc::sync::Arc;
use alloc::vec;
use lazy_static::lazy_static;
use lose_net_stack::results::Packet;
use lose_net_stack::{IPv4, LoseStack, MacAddress, TcpFlags};

mod port_table;
mod socket;
mod tcp;

lazy_static! {
    static ref LOCK_NET_STACK: Arc<NetStack> = Arc::new(NetStack::new());
}

pub struct NetStack(UpIntrFreeCell<LoseStack>);

impl NetStack {
    pub fn new() -> Self {
        unsafe {
            NetStack(UpIntrFreeCell::new(LoseStack::new(
                IPv4::new(10, 0, 2, 15),
                MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
            )))
        }
    }
}

pub fn net_interrupt_handler() {
    let mut recv_buf = vec![0_u8; 1024];

    let len = NET_DEVICE.receive(&mut recv_buf);

    let packet = LOCK_NET_STACK
        .0
        .exclusive_access()
        .analysis(&recv_buf[..len]);

    match packet {
        Packet::ARP(arp_packet) => {
            let lose_stack = LOCK_NET_STACK.0.exclusive_access();
            let reply_packet = arp_packet
                .reply_packet(lose_stack.ip, lose_stack.mac)
                .expect("can't build reply");
            let reply_data = reply_packet.build_data();
            NET_DEVICE.transmit(&reply_data)
        }
        Packet::UDP(udp_packet) => {
            let target = udp_packet.source_ip;
            let l_port = udp_packet.dest_port;
            let r_port = udp_packet.source_port;

            if let Some(source_index) = get_socket(target, l_port, r_port) {
                push_data(source_index, udp_packet.data.to_vec());
            }
        }
        Packet::TCP(tcp_packet) => {
            let target = tcp_packet.source_ip;
            let l_port = tcp_packet.dest_port;
            let r_port = tcp_packet.source_port;
            let flags = tcp_packet.flags;

            if flags.contains(TcpFlags::S) {
                if check_accept(l_port, &tcp_packet).is_some() {
                    let mut reply_packet = tcp_packet.ack();
                    reply_packet.flags = TcpFlags::S | TcpFlags::A;
                    NET_DEVICE.transmit(&reply_packet.build_data());
                }
                return;
            } else if tcp_packet.flags.contains(TcpFlags::F) {
                let reply_packet = tcp_packet.ack();
                NET_DEVICE.transmit(&reply_packet.build_data());

                let mut end_packet = reply_packet.ack();
                end_packet.flags |= TcpFlags::F;
                NET_DEVICE.transmit(&end_packet.build_data());
            } else if tcp_packet.flags.contains(TcpFlags::A) && tcp_packet.data_len == 0 {
                return;
            }

            if let Some(socket_index) = get_socket(target, l_port, r_port) {
                push_data(socket_index, tcp_packet.data.to_vec());
                set_s_a_by_index(socket_index, tcp_packet.seq, tcp_packet.ack);
            }
        }
        _ => {}
    }
}
