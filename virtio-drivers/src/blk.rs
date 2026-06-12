use crate::queue::VirtQueue;
use crate::{AsBuf, Error, Result};
use crate::{Hal, VirtIOHeader};
use bitflags::bitflags;
use core::hint::spin_loop;
use log::info;
use volatile::Volatile;

/// 一个扇区大小
const BLK_SIZE: usize = 512;

bitflags! {
    struct BlkFeature:u64 {
        const BARRIR       = 1 << 0;
        const SIZE_MAX     = 1 << 1;
        const SEG_MAX      = 1 << 2;
        const GEOMETRY     = 1 << 4;
        const RO           = 1 << 5;
        const BLK_SIZE     = 1 << 6;
        const SCSI         = 1 << 7;
        const FLUSH        = 1 << 9;
        const TOPOLOGY     = 1 << 10;
        const CONFIG_WCE   = 1 << 11;
        const DISCARD      = 1 << 13;
        const WRITE_ZEROES = 1 << 14;

        // 以下特征位参考 input.rs 的 Feature 说明
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
/// 块设备配置
///
/// @author: tryte
///
/// @date: 2026/6/12
#[repr(C)]
#[derive(Debug)]
struct BlkConfig {
    /// 总扇区数
    capacity: Volatile<u64>,
    /// 单次 I/O 请求允许的最大字节数
    size_max: Volatile<u32>,
    /// 一个请求最多允许多少个 descriptor
    seg_max: Volatile<u32>,
    /// cylinders / heads / sectors 历史遗留字段，仅兼容旧分区工具
    cylinders: Volatile<u16>,
    heads: Volatile<u8>,
    sectors: Volatile<u8>,
    /// 逻辑块大小
    blk_size: Volatile<u32>,
    /// 物理块大小 = 逻辑块大小 × 2 ^ physical_block_exp
    physical_block_exp: Volatile<u8>,
    /// 逻辑块与物理块的偏移对齐，告诉 OS：partition 起点是否对齐物理块
    alignment_offset: Volatile<u8>,
    /// 最小推荐 I/O 大小
    min_io_size: Volatile<u16>,
    /// 最优 I/O 大小（性能最佳）
    opt_io_size: Volatile<u32>,
    // ..ignored
}

#[repr(C)]
#[derive(Debug)]
enum ReqType {
    In = 0,
    Out = 1,
    Flush = 4,
    Discard = 11,
    WriteZeroes = 13,
}

#[repr(C)]
#[derive(Debug)]
struct BlkReq {
    type_: ReqType,
    reserved: u32,
    sector: u64,
}

unsafe impl AsBuf for BlkReq {}

#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum RespStatus {
    Ok = 0,
    IoErr = 1,
    Unsupported = 2,
    _NotReady = 3,
}

#[repr(C)]
#[derive(Debug)]
pub struct BlkResp {
    status: RespStatus,
}

impl BlkResp {
    pub fn status(&self) -> RespStatus {
        self.status
    }
}

impl Default for BlkResp {
    fn default() -> Self {
        BlkResp {
            status: RespStatus::_NotReady,
        }
    }
}

unsafe impl AsBuf for BlkResp {}

///
/// 虚拟IO设备块
///
/// @author: tryte
///
/// @date: 2026/6/12
pub struct VirtIOBlk<'a, H: Hal> {
    header: &'static mut VirtIOHeader,
    queue: VirtQueue<'a, H>,
    capacity: usize,
}

impl<H: Hal> VirtIOBlk<'_, H> {
    ///
    /// 封装块设备驱动
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/12
    pub fn new(header: &'static mut VirtIOHeader) -> Result<Self> {

        // 初始化驱动
        header.begin_init(|features| {
            let features = BlkFeature::from_bits_truncate(features);
            info!("device features: {:?}", features);
            let supported_features = BlkFeature::empty();
            (features & supported_features).bits()
        });

        // 读取块设备的配置
        let config = unsafe { &mut *(header.config_space() as *mut BlkConfig) };
        info!("config: {:?}", config);
        info!(
            "found a block device of size {}KB",
            // 磁盘大小（KB） = sector 数 × 512 Bytes ÷ 1024
            config.capacity.read() / 2
        );

        // 创建队列大小
        let queue = VirtQueue::new(header, 0, 16)?;
        header.finish_init();

        Ok(VirtIOBlk {
            header,
            queue,
            capacity: config.capacity.read() as usize,
        })
    }

    ///
    /// 应答事件
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/12
    pub fn ack_interrupt(&mut self) -> bool {
        self.header.ack_interrupt()
    }

    ///
    /// 阻塞读取块内容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/12
    pub fn read_block(&mut self, block_id: usize, buf: &mut [u8]) -> Result {
        // 单次读取不能超过一个扇区大小
        assert_eq!(buf.len(), BLK_SIZE);
        // 构建读取请求
        let req = BlkReq {
            type_: ReqType::In,
            reserved: 0,
            sector: block_id as u64,
        };

        // 读取响应
        let mut resp = BlkResp::default();
        // 将写入请求作为输入，读取内容和读取响应作为输出
        self.queue.add(&[req.as_buf()], &[buf, resp.as_buf_mut()])?;
        // 通知设备 队列0 有任务需要处理
        self.header.notify(0);
        // 等待任务返回
        while !self.queue.can_pop() {
            // 让 CPU 更优化的处理空转，而不是激进运行，并不会导致 cpu 暂停或者切换线程
            spin_loop();
        }
        // 弹出事件，回收 desc，读取的内容在 buf 中，因此直接读取 buf 的内容就是硬盘的内容
        self.queue.pop_used()?;
        // 返回设备处理事件状态
        match resp.status {
            RespStatus::Ok => Ok(()),
            _ => Err(Error::IoError),
        }
    }

    ///
    /// 非阻塞读取块内容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/12
    pub unsafe fn read_block_nb(
        &mut self,
        block_id: usize,
        buf: &mut [u8],
        resp: &mut BlkResp,
    ) -> Result<u16> {
        // 单次读取不能超过一个扇区大小
        assert_eq!(buf.len(), BLK_SIZE);
        // 构建读取请求
        let req = BlkReq {
            type_: ReqType::In,
            reserved: 0,
            sector: block_id as u64,
        };

        // 将写入请求作为输入，读取内容和读取响应作为输出
        let token = self.queue.add(&[req.as_buf()], &[buf, resp.as_buf_mut()])?;
        // 通知设备 队列0 有任务需要处理
        self.header.notify(0);
        // 返回任务ID，不等待结果直接返回
        Ok(token)
    }

    ///
    /// 阻塞写入块内容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/12
    pub fn write_block(&mut self, block_id: usize, buf: &[u8]) -> Result {
        // 单次写入不能超过一个扇区大小
        assert_eq!(buf.len(), BLK_SIZE);

        // 构建写入请求
        let req = BlkReq {
            type_: ReqType::Out,
            reserved: 0,
            sector: block_id as u64,
        };

        // 写入响应
        let mut resp = BlkResp::default();
        // 将写入请求和写入内容作为输入，写入响应作为输出
        self.queue.add(&[req.as_buf(), buf], &[resp.as_buf_mut()])?;
        // 通知设备 队列0 有任务需要处理
        self.header.notify(0);
        // 等待任务返回
        while !self.queue.can_pop() {
            spin_loop();
        }
        // 弹出事件，回收 desc
        self.queue.pop_used()?;
        // 返回设备处理事件状态
        match resp.status {
            RespStatus::Ok => Ok(()),
            _ => Err(Error::IoError),
        }
    }

    ///
    /// 非阻塞写入块内容
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/12
    pub unsafe fn write_block_nb(
        &mut self,
        block_id: usize,
        buf: &[u8],
        resp: &mut BlkResp,
    ) -> Result<u16> {
        // 单次写入不能超过一个扇区大小
        assert_eq!(buf.len(), BLK_SIZE);
        // 构建写入请求
        let req = BlkReq {
            type_: ReqType::Out,
            reserved: 0,
            sector: block_id as u64,
        };
        // 将写入请求和写入内容作为输入，写入响应作为输出
        let token = self.queue.add(&[req.as_buf(), buf], &[resp.as_buf_mut()])?;
        // 通知设备 队列0 有任务需要处理
        self.header.notify(0);
        // 返回任务ID，不等待结果直接返回
        Ok(token)
    }

    ///
    /// 弹出已处理事件
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/12
    pub fn pop_used(&mut self) -> Result<u16> {
        self.queue.pop_used().map(|p| p.0)
    }

    ///
    /// 获取队列长度
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/12
    pub fn virt_queue_size(&self) -> u16 {
        self.queue.size()
    }
}
