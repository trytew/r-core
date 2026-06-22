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

pub use port_table::{accept, listen, port_acceptable, PortFd};

lazy_static! {
    static ref LOCK_NET_STACK: Arc<NetStack> = Arc::new(NetStack::new());
}

///
/// 网络协议栈
///
/// @author: tryte
///
/// @date: 2026/6/22
pub struct NetStack(UpIntrFreeCell<LoseStack>);

impl NetStack {
    ///
    /// 创建网络协议栈
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/22
    pub fn new() -> Self {
        unsafe {
            NetStack(UpIntrFreeCell::new(LoseStack::new(
                IPv4::new(10, 0, 2, 15),
                MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
            )))
        }
    }
}

///
/// 网络外设中断处理
///
/// @author: tryte
///
/// @date: 2026/6/22
pub fn net_interrupt_handler() {
    // 接收数据 buffer
    let mut recv_buf = vec![0_u8; 1024];

    // 接收数据
    let len = NET_DEVICE.receive(&mut recv_buf);

    // 解析数据包
    let packet = LOCK_NET_STACK
        .0
        .exclusive_access()
        .analysis(&recv_buf[..len]);

    match packet {
        // ARP 包
        Packet::ARP(arp_packet) => {
            // ARP 广播，询问当前网卡的 MAC 地址，数据包的发送需要使用 MAC 地址，应答 ARP 广播告知对端网卡 MAC 地址，使对端可对网卡发送数据
            // 构建 ARP 应答数据包
            let lose_stack = LOCK_NET_STACK.0.exclusive_access();
            let reply_packet = arp_packet
                .reply_packet(lose_stack.ip, lose_stack.mac)
                .expect("can't build reply");
            let reply_data = reply_packet.build_data();
            // 发送数据
            NET_DEVICE.transmit(&reply_data)
        }
        // UDP 包
        Packet::UDP(udp_packet) => {
            let target = udp_packet.source_ip;
            let l_port = udp_packet.dest_port;
            let r_port = udp_packet.source_port;

            if let Some(source_index) = get_socket(target, l_port, r_port) {
                push_data(source_index, udp_packet.data.to_vec());
            }
        }
        // TCP 包
        Packet::TCP(tcp_packet) => {
            // 客户端IP
            let target = tcp_packet.source_ip;
            // 客户端目标端口好，即本地监听端口
            let l_port = tcp_packet.dest_port;
            // 客户端端口号
            let r_port = tcp_packet.source_port;
            // TCP控制位：
            // SYN = 建立连接
            // ACK = 确认
            // FIN = 关闭连接
            // RST = 重置连接
            // PSH = 推送数据
            // URG = 紧急数据
            let flags = tcp_packet.flags;

            if flags.contains(TcpFlags::S) {
                // 建立连接
                if check_accept(l_port, &tcp_packet).is_some() {
                    let mut reply_packet = tcp_packet.ack();
                    reply_packet.flags = TcpFlags::S | TcpFlags::A;
                    NET_DEVICE.transmit(&reply_packet.build_data());
                }
                return;
            } else if tcp_packet.flags.contains(TcpFlags::F) {
                // 关闭连接
                let reply_packet = tcp_packet.ack();
                NET_DEVICE.transmit(&reply_packet.build_data());

                let mut end_packet = reply_packet.ack();
                end_packet.flags |= TcpFlags::F;
                NET_DEVICE.transmit(&end_packet.build_data());
            } else if tcp_packet.flags.contains(TcpFlags::A) && tcp_packet.data_len == 0 {
                // 客户端 ACK 响应
                return;
            }

            if let Some(socket_index) = get_socket(target, l_port, r_port) {
                // 记录队列数据
                push_data(socket_index, tcp_packet.data.to_vec());
                // 记录序列号和应答号
                set_s_a_by_index(socket_index, tcp_packet.seq, tcp_packet.ack);
            }
        }
        _ => {}
    }
}
