use crate::queue::VirtQueue;
use crate::{AsBuf, Result};
use crate::{Hal, VirtIOHeader};
use bitflags::bitflags;
use core::hint::spin_loop;
use core::mem::MaybeUninit;
use log::{debug, info};
use volatile::{ReadOnly, Volatile};

const QUEUE_RECEIVE: usize = 0;
const QUEUE_TRANSMIT: usize = 1;

bitflags! {
    struct Features:u64 {
        const CSUM = 1 << 0;
        const GUEST_CSUM = 1 << 1;
        const CTRL_GUEST_OFFLOADS = 1 << 2;
        const MTU = 1 << 3;
        const MAC = 1 << 5;
        const GSO = 1 << 6;
        const GUEST_TSO4 = 1 << 7;
        const GUEST_TSO6 = 1 << 8;
        const GUEST_ECN = 1 << 9;
        const GUEST_UFO = 1 << 10;
        const HOST_TSO4 = 1 << 11;
        const HOST_TSO6 = 1 << 12;
        const HOST_ECN = 1 << 13;
        const HOST_UFO = 1 << 14;
        const MRG_RXBUF = 1 << 15;
        const STATUS = 1 << 16;
        const CTRL_VQ = 1 << 17;
        const CTRL_RQ = 1 << 18;
        const CTRL_VLAN = 1 << 19;
        const CTRL_RX_EXTRA = 1 << 20;
        const GUEST_ANNOUNCE = 1 << 21;
        const MQ = 1 << 22;
        const CTL_MAC_ADDR = 1 << 23;

        // 以下特征位参考 input.rs 的 Feature 说明
        const RING_INDIRECT_DESC = 1 << 28;
        const RING_EVENT_IDX     = 1 << 29;
        const VERSION_1          = 1 << 32;
    }
}

type EthernetAddress = [u8; 6];

bitflags! {
    struct Status:u16 {
        const LINK_UP = 1;
        const ANNOUNCE = 2;
    }
}

struct Config {
    mac: ReadOnly<EthernetAddress>,
    status: ReadOnly<Status>,
}


bitflags! {
    struct Flags:u8 {
        const NEEDS_CSUM = 1;
        const DATA_VALID = 2;
        const RSC_INFO = 4;
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum GsoType {
    None = 0,
    TcpV4 = 1,
    Udp = 3,
    TcpV6 = 4,
    Ecn = 0x80,
}

#[repr(C)]
#[derive(Debug)]
struct Header {
    flags: Volatile<Flags>,
    gso_type: Volatile<GsoType>,
    hdr_len: Volatile<u16>,
    gso_size: Volatile<u16>,
    c_sum_start: Volatile<u16>,
    c_sum_offset: Volatile<u16>,
}

unsafe impl AsBuf for Header {}

///
/// 网卡驱动
///
/// @author: tryte
///
/// @date: 2026/6/17
pub struct VirtIONet<'a, H: Hal> {
    header: &'static mut VirtIOHeader,
    mac: EthernetAddress,
    recv_queue: VirtQueue<'a, H>,
    send_queue: VirtQueue<'a, H>,
}

impl<H: Hal> VirtIONet<'_, H> {
    ///
    /// 创建网卡驱动
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/17
    pub fn new(header: &'static mut VirtIOHeader) -> Result<Self> {
        header.begin_init(|features: u64| {
            let features = Features::from_bits_truncate(features);
            info!("device features: {:?}",features);
            let supported_features = Features::MAC | Features::STATUS;
            (features & supported_features).bits()
        });

        let config = unsafe { &mut *(header.config_space() as *mut Config) };
        let mac = config.mac.read();
        debug!("Got Mac={:?}, status={:?}",mac,config.status.read());

        let queue_num = 2;
        let recv_queue = VirtQueue::new(header, QUEUE_RECEIVE, queue_num)?;
        let send_queue = VirtQueue::new(header, QUEUE_TRANSMIT, queue_num)?;

        header.finish_init();

        Ok(VirtIONet {
            header,
            mac,
            recv_queue,
            send_queue,
        })
    }

    ///
    /// 应答设备
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/17
    pub fn ack_interrupt(&mut self) -> bool {
        self.header.ack_interrupt()
    }

    ///
    /// 获取 mac 地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/17
    pub fn mac(&self) -> EthernetAddress {
        self.mac
    }

    ///
    /// 是否可以发送
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/17
    pub fn can_send(&self) -> bool {
        self.send_queue.available_desc() >= 2
    }

    ///
    /// 是否可接收
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/17
    pub fn can_recv(&self) -> bool {
        self.recv_queue.can_pop()
    }

    ///
    /// 接收数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/17
    pub fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut header = MaybeUninit::<Header>::uninit();
        let header_buf = unsafe { (*header.as_mut_ptr()).as_buf_mut() };
        self.recv_queue.add(&[], &[header_buf, buf])?;
        self.header.notify(QUEUE_RECEIVE as u32);
        while !self.recv_queue.can_pop() {
            spin_loop();
        }

        let (_, len) = self.recv_queue.pop_used()?;
        Ok(len as usize - size_of::<Header>())
    }

    ///
    /// 发送数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/17
    pub fn send(&mut self, buf: &[u8]) -> Result {
        let header = unsafe { MaybeUninit::<Header>::zeroed().assume_init() };
        self.send_queue.add(&[header.as_buf(), buf], &[])?;
        self.header.notify(QUEUE_TRANSMIT as u32);
        while !self.send_queue.can_pop() {
            spin_loop();
        }
        self.send_queue.pop_used()?;
        Ok(())
    }
}