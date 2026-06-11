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
    struct Feature:u64 {
        const NOTIFY_ON_EMPTY    = 1 << 24;
        const ANY_LAYOUT         = 1 << 27;
        const RING_INDIRECT_DESC = 1 << 28;
        const RING_EVENT_IDX     = 1 << 29;
        const UNUSED             = 1 << 30;
        const VERSION_1          = 1 << 32;

        const ACCESS_PLATFORM   = 1 << 33;
        const RING_PACKED       = 1 << 34;
        const IN_ORDER          = 1 << 35;
        const ORDER_PLATFORM    = 1 << 36;
        const SR_IOV            = 1 << 37;
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
            info!("Device features: {:?}", features);
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
            // 将数据缓冲区放回待处理任务队列，这个时候 event 的数据是有可能被设备覆盖的，只是 Copy 的事件很短，没有做考虑
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
