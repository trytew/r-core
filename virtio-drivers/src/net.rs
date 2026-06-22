use crate::queue::VirtQueue;
use crate::{AsBuf, Result};
use crate::{Hal, VirtIOHeader};
use bitflags::bitflags;
use core::hint::spin_loop;
use core::mem::MaybeUninit;
use log::{debug, info};
use volatile::{ReadOnly, Volatile};

/// 数据接收
const QUEUE_RECEIVE: usize = 0;

/// 数据发送
const QUEUE_TRANSMIT: usize = 1;

bitflags! {
    struct Features:u64 {
        /// 正常发送一个 TCP 包时，操作系统需要做很多事：
        /// ```shell
        /// 应用程序
        ///     ↓
        /// 构造TCP头
        ///     ↓
        /// 计算TCP校验和
        ///     ↓
        /// 计算IP校验和
        ///     ↓
        /// 如果超过MTU则拆包
        ///     ↓
        /// 发送给网卡
        /// ```
        /// 这些工作都要消耗 CPU。于是出现了各种 Offload（卸载） 技术，意思是原来CPU干的活交给网卡干
        ///
        /// TSO：TCP分段卸载。假设 MTU 只有 1.5KB，应用需要发送 64KB 数据。
        /// 如果没有 TSO，那么需要 CPU 自行拆包构造 TCP 头并技术校验和，开启 TSO 后由网卡自行拆分计算

        /// 校验和卸载
        /// 设备支持发送数据校验和卸载，开启后网卡自行计算TCP校验和
        const CSUM = 1 << 0;
        /// 设备支持接收数据校验和卸载格式的数据
        const GUEST_CSUM = 1 << 1;
        /// 运行过程中可以动态开关这些 offload 功能，通过 control virtqueue 配置
        const CTRL_GUEST_OFFLOADS = 1 << 2;
        /// 设备提供 MTU 信息
        const MTU = 1 << 3;
        /// 设备配置空间中包含 MAC 地址
        const MAC = 1 << 5;

        /// 大包分段卸载
        /// 通用分段卸载，是 Linux 提出的一个抽象层。网卡支持 GSO → TSO，最终由网卡拆。如果网卡不支持 GSO → CPU拆，操作系统自己拆
        const GSO = 1 << 6;
        /// Guest 能接收 IPv4 TSO 包
        const GUEST_TSO4 = 1 << 7;
        /// Guest 能接收 IPv6 TSO 包
        const GUEST_TSO6 = 1 << 8;
        /// 支持 ECN
        const GUEST_ECN = 1 << 9;
        /// 支持 UDP Fragmentation Offload，大 UDP 包由设备拆分
        const GUEST_UFO = 1 << 10;
        /// Host 支持 IPv4 TSO
        const HOST_TSO4 = 1 << 11;
        /// Host 支持 IPv6 TSO
        const HOST_TSO6 = 1 << 12;
        /// Host 支持 ECN。ECN：显式拥塞通知。
        /// 常规 TCP 发现网络拥塞的时候会直接丢包：发送方 -> 路由器 -> 丢包，发送方收到超时或者 3 次重复ACK，然后减速。
        /// 开启 ECN 后：发送方 -> 路由器 -> 路由器发现队列快满了，给数据包打上 CE 标记 -> 接收方 -> ACK中通知发送方，然后发送方主动减速
        const HOST_ECN = 1 << 13;
        /// Host 支持 UDP Fragmentation Offload
        const HOST_UFO = 1 << 14;

        /// 支持接收数据包合并，开启后可将数据包合并放入多个 desc
        const MRG_RXBUF = 1 << 15;
        /// 链路状态
        const STATUS = 1 << 16;
        /// 控制队列，支持 Control VirtQueue，会额外多出一个队列
        /// rx queue
        /// tx queue
        /// control queue
        const CTRL_VQ = 1 << 17;
        /// Receive Queue 控制。可动态开启/关闭某些 RX Queue。通常与 MQ 配合。
        const CTRL_RQ = 1 << 18;
        /// 支持通过 Control Queue 配置 VLAN，如：
        /// 允许 VLAN 100
        /// 拒绝 VLAN 200
        const CTRL_VLAN = 1 << 19;
        /// 支持额外的 RX Filter
        const CTRL_RX_EXTRA = 1 << 20;
        /// 支持网络重新公告。例如 VM 迁移后发送 Gratuitous ARP。告诉交换机我还活着,MAC地址在这里
        const GUEST_ANNOUNCE = 1 << 21;
        /// 多队列，多个 CPU 可以同时处理网络包
        const MQ = 1 << 22;
        /// 修改 MAC 地址，允许通过 Control Queue 修改 MAC 无需重启设备
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

///
/// 包分段类型
///
/// @author: tryte
///
/// @date: 2026/6/22
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum GsoType {
    /// 无需分段
    None = 0,
    /// ipv4 tcp 大包，需要 TSO
    TcpV4 = 1,
    /// udp
    Udp = 3,
    /// ipv6 tcp 大包，需要 TSO
    TcpV6 = 4,
    /// 是否拥塞
    Ecn = 0x80,
}

///
/// 数据包头
///
/// @author: tryte
///
/// @date: 2026/6/22
#[repr(C)]
#[derive(Debug)]
struct Header {
    /// 标识是否需要检验或计算校验和
    flags: Volatile<Flags>,
    /// 包是否需要分段，分段类型是什么
    gso_type: Volatile<GsoType>,
    /// 网络头总长度
    hdr_len: Volatile<u16>,
    /// 拆包后的 Payload 大小
    gso_size: Volatile<u16>,
    /// 校验和开始计算的位置
    c_sum_start: Volatile<u16>,
    /// 校验和字段距离 c_sum_start 的偏移
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
    /// 驱动头
    header: &'static mut VirtIOHeader,
    /// 网卡 MAC 地址
    mac: EthernetAddress,
    /// 接收队列
    recv_queue: VirtQueue<'a, H>,
    /// 发送队列
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
        // 设备初始化
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
        // 包数据头
        let mut header = MaybeUninit::<Header>::uninit();
        // 读取数据包头 buffer
        let header_buf = unsafe { (*header.as_mut_ptr()).as_buf_mut() };
        // 读取数据
        self.recv_queue.add(&[], &[header_buf, buf])?;
        // 接受数据头通知
        self.header.notify(QUEUE_RECEIVE as u32);
        // 等待数据响应
        while !self.recv_queue.can_pop() {
            spin_loop();
        }

        // 回收 desc
        let (_, len) = self.recv_queue.pop_used()?;
        // 返回数据长度
        Ok(len as usize - size_of::<Header>())
    }

    ///
    /// 发送数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/17
    pub fn send(&mut self, buf: &[u8]) -> Result {
        // 包装数据头
        let header = unsafe { MaybeUninit::<Header>::zeroed().assume_init() };
        // 向网卡写入数据
        self.send_queue.add(&[header.as_buf(), buf], &[])?;
        // 发送数据写入通知
        self.header.notify(QUEUE_TRANSMIT as u32);
        // 等待数据发送成功
        while !self.send_queue.can_pop() {
            spin_loop();
        }
        // 回收 desc
        self.send_queue.pop_used()?;
        Ok(())
    }
}