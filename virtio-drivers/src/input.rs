use crate::hal::Hal;
use crate::header::VirtIOHeader;
use crate::queue::VirtQueue;
use crate::AsBuf;
use crate::Result;
use alloc::boxed::Box;
use bitflags::bitflags;
use log::info;
use volatile::{ReadOnly, WriteOnly};

const QUEUE_SIZE: usize = 32;

const QUEUE_EVENT: usize = 0;

const QUEUE_STATUS: usize = 1;

bitflags! {
    ///
    /// 输入设备功能特性位
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/12
    struct Feature:u64 {
        /// 当 avail ring 被消费空时通知驱动
        const NOTIFY_ON_EMPTY    = 1 << 24;
        /// 支持任意 Descriptor 链布局（兼容旧版 VirtIO）
        const ANY_LAYOUT         = 1 << 27;
        /// 支持间接 Descriptor，可用一个 Desc 引用一张 Desc 表
        /// 正常情况：Descriptor Table 0、1、2、3...
        /// 如果一次请求需要 100 个 desc 需要占 100 个槽，开启后：
        /// Descriptor Table 0、1、2 ... 99
        ///                  ↓
        ///            Indirect Table
        /// 每个 Descriptor 都可以维护一个间接表，减少 Descriptor Table 消耗
        const RING_INDIRECT_DESC = 1 << 28;
        /// 支持 Event Index，减少不必要的中断通知
        /// 默认：每完成一次请求 -> 中断一次，开启后：
        /// 完成到指定 idx 才中断
        /// 例如：处理到 used_idx=100 再通知，减少中断风暴
        const RING_EVENT_IDX     = 1 << 29;
        /// 保留位，未使用
        const UNUSED             = 1 << 30;
        /// 表示是否符合 VirtIO 1.0+ 规范（现代 VirtIO 设备）
        const VERSION_1          = 1 << 32;
        /// 支持平台 DMA/IOMMU 地址转换，设备访问内存是否需要经过平台 DMA/IOMMU，驱动必须根据这个标志位正确处理 DMA 地址转换
        const ACCESS_PLATFORM   = 1 << 33;
        /// 支持 Packed VirtQueue 格式
        ///
        /// 传统 VirtQueue：
        ///     Descriptor Table
        ///     Avail Ring
        ///     Used Ring
        ///
        /// Packed VirtQueue：
        ///  +------+    +-----------+
        ///  | desc |    | addr      |
        ///  |      | -> | len       |
        ///  |      |    | flags     |
        ///  |      |    | owner bit | -> owner bit 类似：0 = Driver拥有；1 = Device拥有
        ///  +------+    +-----------+
        ///  | desc |
        ///  +------+
        ///
        /// Packed VirtQueue 将三个队列融合成一个队列，每个队列元素包含addr、len、flags、owner bit，
        /// 这样设备在获取驱动任务的时候就不需要：读 avail_idx -> 读 avail ring[idx] -> 读 desc -> 写 used
        /// 可以提高 cache 缓存命中率和减少内存读取存储操作
        ///
        const RING_PACKED       = 1 << 34;
        /// 保证请求按提交顺序完成
        const IN_ORDER          = 1 << 35;
        /// 遵循平台内存排序规则，与内存屏障有关，表示设备是否遵循平台内存排序规则
        const ORDER_PLATFORM    = 1 << 36;
        /// 支持 SR-IOV 虚拟化功能，即：一个物理设备 -> 多个虚拟设备，如：
        /// 物理网卡
        /// ├─ VF0
        /// ├─ VF1
        /// ├─ VF2
        /// 常见于高性能虚拟化
        const SR_IOV            = 1 << 37;
        /// 通知时可携带额外队列信息，优化 MMIO Notify。传统：notify -> 设备再查队列信息，开启后：notify 时直接带队列信息。例如：
        /// queue id
        /// next avail idx
        /// 减少一次内存访问
        const NOTIFICATION_DATA = 1 << 38;
    }
}

///
/// 设备设置
///
/// @author: tryte
///
/// @date: 2026/6/10
#[repr(C)]
struct Config {
    select: WriteOnly<u8>,
    sub_sel: WriteOnly<u8>,
    size: ReadOnly<u8>,
    _reversed: [ReadOnly<u8>; 5],
    data: ReadOnly<[u8; 128]>,
}

///
/// 输入事件
///
/// @author: tryte
///
/// @date: 2026/6/9
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct InputEvent {
    /// 事件类型
    pub event_type: u16,
    /// 事件代码
    pub code: u16,
    /// 事件值
    pub value: u32,
}

unsafe impl AsBuf for InputEvent {}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum InputConfigSelect {
    IdName = 0x01,
    IdSerial = 0x02,
    IdDevIds = 0x03,
    PropBits = 0x10,
    EvBits = 0x11,
    AbsInfo = 0x12,
}

///
/// 虚拟IO输入
///
/// @author: tryte
///
/// @date: 2026/6/9
pub struct VirtIOInput<'a, H: Hal> {
    header: &'static mut VirtIOHeader,
    event_queue: VirtQueue<'a, H>,
    status_queue: VirtQueue<'a, H>,
    event_buf: Box<[InputEvent; 32]>,
}

impl<'a, H: Hal> VirtIOInput<'a, H> {
    ///
    /// 封装输入驱动
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn new(header: &'static mut VirtIOHeader) -> Result<Self> {
        // 创建空白事件队列
        let mut event_buf = Box::new([InputEvent::default(); QUEUE_SIZE]);
        // 初始化IO外设
        header.begin_init(|features| {
            let features = Feature::from_bits_truncate(features);
            info!("device features: {:?}", features);
            let supported_features = Feature::empty();
            (features & supported_features).bits()
        });

        // 创建事件队列
        let mut event_queue = VirtQueue::new(header, QUEUE_EVENT, QUEUE_SIZE as u16)?;
        // 创建状态通知队列
        let status_queue = VirtQueue::new(header, QUEUE_STATUS, QUEUE_SIZE as u16)?;

        for (i, event) in event_buf.as_mut().iter_mut().enumerate() {
            // 填充输出事件
            let token = event_queue.add(&[], &[event.as_buf_mut()])?;
            assert_eq!(token, i as u16);
        }

        // 完成设备初始化
        header.finish_init();

        Ok(VirtIOInput {
            header,
            event_queue,
            status_queue,
            event_buf,
        })
    }

    ///
    /// 事件应答
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn ack_interrupt(&mut self) -> bool {
        self.header.ack_interrupt()
    }

    ///
    /// 读取已完成任务
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn pop_pending_event(&mut self) -> Option<InputEvent> {
        // 弹出任务
        if let Ok((token, _)) = self.event_queue.pop_used() {
            // 读取任务输出数据
            let event = &mut self.event_buf[token as usize];
            // 将数据缓冲区放回待处理任务队列，这个时候 event 的数据是有可能被设备覆盖的，因为 event 是作为 desc 的输出 buffer，
            // 只是 Copy 的时间很短，没有做考虑，也意味着键盘/鼠标的事件对操作系统来说是允许丢失的
            if self.event_queue.add(&[], &[event.as_buf_mut()]).is_ok() {
                // InputEvent 实现了 Copy 语义，这里的数据会被 Copy 返回
                return Some(*event);
            }
        }
        None
    }

    ///
    ///
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn query_config_select(
        &mut self,
        select: InputConfigSelect,
        sub_sel: u8,
        out: &mut [u8],
    ) -> u8 {
        let config = unsafe { &mut *(self.header.config_space() as *mut Config) };
        config.select.write(select as u8);
        config.sub_sel.write(sub_sel);
        let size = config.size.read();
        let data = config.data.read();
        out[..size as usize].copy_from_slice(&data[..size as usize]);
        size
    }

    ///
    /// 返回队列长度
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/9
    pub fn virt_queue_size(&self) -> u16 {
        QUEUE_SIZE as u16
    }
}
