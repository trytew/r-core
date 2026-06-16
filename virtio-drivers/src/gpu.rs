use crate::hal::DMA;
use crate::queue::VirtQueue;
use crate::{pages, Error, Hal, VirtIOHeader};
use crate::{Result, PAGE_SIZE};
use bitflags::bitflags;
use core::fmt;
use core::fmt::Formatter;
use core::hint::spin_loop;
use log::info;
use volatile::{ReadOnly, Volatile, WriteOnly};

/// GPU 控制队列
const QUEUE_TRANSMIT: usize = 0;
/// GPU 光标队列
const QUEUE_CURSOR: usize = 1;

/// 显示器ID
const SCANOUT_ID: u32 = 0;
/// 2D 图像资源编号
const RESOURCE_ID_FB: u32 = 0xBABE;
/// 光标图形资源编号
const RESOURCE_ID_CURSOR: u32 = 0xDADE;

///
/// 光标信息
///
/// @author: tryte
///
/// @date: 2026/6/15
const CURSOR_RECT: Rect = Rect {
    x: 0,
    y: 0,
    width: 64,
    height: 64,
};

bitflags! {
    struct Features:u64 {
        const VIRGL = 1 << 0;
        const EDID = 1 << 1;
        const F_CURSOR = 1 << 4;

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

#[repr(C)]
struct Config {
    event_read: ReadOnly<u32>,
    event_clear: WriteOnly<u32>,
    num_scanout: Volatile<u32>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("events_read", &self.event_read)
            .field("num_scanout", &self.num_scanout)
            .finish()
    }
}

///
/// GPU 设备指令id
///
/// @author: tryte
///
/// @date: 2026/6/15
#[repr(u32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Command {
    /// 获取显示器信息
    GetDisplayInfo = 0x100,
    /// 创建 2D 图像资源
    ResourceCreate2d = 0x101,
    /// 删除 2D 图像资源
    ResourceUnRef = 0x102,
    /// 显示输出设置，设置把某个资源（图片/帧缓冲）显示到屏幕的某一块区域
    SetScanout = 0x103,
    /// 通知 GPU 刷新屏幕
    ResourceFlush = 0x104,
    /// 将修改的缓存内容同步到 GPU Resource
    TransferToHost2d = 0x105,
    /// 绑定图像资源内存地址
    ResourceAttachBacking = 0x106,
    /// 解绑图像资源内存地址
    ResourceDetachBacking = 0x107,
    GetCapSetInfo = 0x108,
    GetCapSet = 0x109,
    GetEdId = 0x10A,

    /// 更新光标图像，并设置位置
    UpdateCursor = 0x300,
    /// 只移动光标位置，不修改光标图像
    MoveCursor = 0x301,

    OkNoData = 0x1100,
    OkDisplayInfo = 0x1101,
    OkCapSetInfo = 0x1102,
    OkCapSet = 0x1103,
    OkEdId = 0x1104,

    ErrUnSpec = 0x1200,
    ErrOutOfMemory = 0x1201,
    ErrInvalidScanoutId = 0x1202,
}

///
/// 控制命令公共头
///
/// @author: tryte
///
/// @date: 2026/6/15
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CtrlHeader {
    /// 指令ID
    hdr_type: Command,
    /// 附加标志位，控制命令行为
    flags: u32,
    /// 请求编号，用于异步完成通知
    fence_id: u64,
    /// GPU 上下文 ID，用于区分不同 GPU 上下文
    ctx_id: u32,
    /// 对齐填充
    _padding: u32,
}

impl CtrlHeader {
    ///
    /// 构造请求
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn with_type(hdr_type: Command) -> CtrlHeader {
        CtrlHeader {
            hdr_type,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            _padding: 0,
        }
    }

    ///
    /// 校验响应数据
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn check_type(&self, expected: Command) -> Result {
        if self.hdr_type == expected {
            Ok(())
        } else {
            Err(Error::IoError)
        }
    }
}

///
/// 显示器信息查询响应
///
/// @author: tryte
///
/// @date: 2026/6/15
#[repr(C)]
#[derive(Debug)]
struct RespDisplayInfo {
    header: CtrlHeader,
    /// 显示器区域，实际上从 rect 字段开始应该是一个数组，用于多个显示器的信息接收，这里只是简化了
    rect: Rect,
    /// 显示器是否可用
    enabled: u32,
    /// 显示器的额外属性标志
    flags: u32,
}

///
/// 每个像素存储方式
///
/// @author: tryte
///
/// @date: 2026/6/15
#[repr(u32)]
#[derive(Debug)]
enum Format {
    /// Byte0 = Blue
    /// Byte1 = Green
    /// Byte2 = Red
    /// Byte3 = Alpha（透明度）
    B8G8R8A8UNORM = 1,
}

///
/// 创建 2D 图像资源请求
///
/// @author: tryte
///
/// @date: 2026/6/15
#[repr(C)]
#[derive(Debug)]
struct StResourceCreate2D {
    /// 请求头
    header: CtrlHeader,
    /// 2D 图像资源编号，后续可使用该编号操作图形资源
    resource_id: u32,
    /// 像素格式
    format: Format,
    /// 图像宽度
    width: u32,
    /// 图像高度
    height: u32,
}

///
/// 设置图像资源数据存放内存
///
/// @author: tryte
///
/// @date: 2026/6/15
#[repr(C)]
#[derive(Debug)]
struct StResourceAttachBacking {
    /// 请求头
    header: CtrlHeader,
    /// 图像资源编号
    resource_id: u32,
    /// 表示后面有多少个内存段，理论上可以：
    /// 资源1
    ///  ├─ 0x10000000 长度4KB
    ///  ├─ 0x20000000 长度4KB
    ///  ├─ 0x30000000 长度4KB
    /// 为了简化直接使用一块连续的内存
    nr_entries: u32, // 总是1
    /// 绑定内存的起始物理地址
    addr: u64,
    /// 绑定内存的长度
    length: u32,
    /// 填充位
    _padding: u32,
}

///
/// 把某个资源（图片/帧缓冲）显示到屏幕的某一块区域请求
///
/// @author: tryte
///
/// @date: 2026/6/15
#[repr(C)]
#[derive(Debug)]
struct StSetScanout {
    /// 请求头
    header: CtrlHeader,
    /// 显示区域
    rect: Rect,
    /// 第几个显示器输出
    scanout_id: u32,
    /// 要显示哪一个图像资源
    resource_id: u32,
}

///
/// 把 CPU 这边写好的像素数据，从内存“搬到 GPU 可见的资源里”
///
/// @author: tryte
///
/// @date: 2026/6/15
#[repr(C)]
#[derive(Debug)]
struct StTransferToHost2D {
    /// 请求头
    header: CtrlHeader,
    /// 本次更新“哪一块区域的像素”
    rect: Rect,
    /// 在 backing memory 里的起始偏移
    offset: u64,
    /// 图像资源ID
    resource_id: u32,
    _padding: u32,
}

#[repr(C)]
#[derive(Debug)]
struct StResourceFlush {
    header: CtrlHeader,
    rect: Rect,
    resource_id: u32,
    _padding: u32,
}

///
/// 光标位置
///
/// @author: tryte
///
/// @date: 2026/6/15
#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct CursorPos {
    /// 显示器ID
    scanout_id: u32,
    x: u32,
    y: u32,
    _padding: u32,
}

///
/// 更新光标位置请求
///
/// @author: tryte
///
/// @date: 2026/6/15
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct StUpdateCursor {
    /// 请求头
    header: CtrlHeader,
    /// 更新位置
    pos: CursorPos,
    /// 图像资源ID
    resource_id: u32,
    /// X轴，鼠标点击“精确点”在图像中的位置，即光标图像里的哪一个点，被当作真实光标位置
    hot_x: u32,
    /// Y轴，鼠标点击“精确点”在图像中的位置，即光标图像里的哪一个点，被当作真实光标位置
    hot_y: u32,
    _padding: u32,
}

///
/// 显示区域
///
/// @author: tryte
///
/// @date: 2026/6/15
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
struct Rect {
    /// x
    x: u32,
    /// y
    y: u32,
    /// 宽度
    width: u32,
    /// 高度
    height: u32,
}

///
/// 虚拟IO GPU
///
/// @author: tryte
///
/// @date: 2026/6/13
pub struct VirtIOGpu<'a, H: Hal> {
    /// GPU 设备头
    header: &'static mut VirtIOHeader,
    /// 显示区域
    rect: Rect,
    // 显示器绑定的图像资源内存操作地址
    frame_buffer_dma: Option<DMA<H>>,
    // 光标绑定的图像资源内存操作地址
    cursor_buffer_dma: Option<DMA<H>>,
    /// 控制任务队列
    control_queue: VirtQueue<'a, H>,
    /// 光标任务队列
    cursor_queue: VirtQueue<'a, H>,
    /// 队列内存
    queue_buf_dma: DMA<H>,
    /// 发送队列内存
    queue_buf_send: &'a mut [u8],
    /// 接收队列内存
    queue_buf_recv: &'a mut [u8],
}

impl<H: Hal> VirtIOGpu<'_, H> {
    ///
    /// 封装 GPU 驱动
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/13
    pub fn new(header: &'static mut VirtIOHeader) -> Result<Self> {
        // 设置头信息
        header.begin_init(|features| {
            let features = Features::from_bits_truncate(features);
            info!("device features: {:?}", features);
            let supported_features = Features::empty();
            (features & supported_features).bits()
        });

        // 读取配置信息
        let config = unsafe { &mut *(header.config_space() as *mut Config) };
        info!("config: {:?}",config);

        // 创建 GPU 控制队列
        let control_queue = VirtQueue::new(header, QUEUE_TRANSMIT, 2)?;
        // 创建光标队列
        let cursor_queue = VirtQueue::new(header, QUEUE_CURSOR, 2)?;

        // 任务队列分配内存
        let queue_buf_dma = DMA::new(2)?;
        let queue_buf_send = unsafe { &mut queue_buf_dma.as_buf()[..PAGE_SIZE] };
        let queue_buf_recv = unsafe { &mut queue_buf_dma.as_buf()[PAGE_SIZE..] };

        // 初始化结束
        header.finish_init();

        Ok(VirtIOGpu {
            header,
            frame_buffer_dma: None,
            cursor_buffer_dma: None,
            rect: Rect::default(),
            control_queue,
            cursor_queue,
            queue_buf_dma,
            queue_buf_send,
            queue_buf_recv,
        })
    }

    ///
    /// 应答事件
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/13
    pub fn ack_interrupt(&mut self) -> bool {
        self.header.ack_interrupt()
    }

    ///
    /// 返回宽高
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/13
    pub fn resolution(&self) -> (u32, u32) {
        (self.rect.width, self.rect.height)
    }

    ///
    /// 设置缓存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/13
    pub fn setup_framebuffer(&mut self) -> Result<&mut [u8]> {
        // 获取显示器信息
        let display_info = self.get_display_info()?;
        info!("=> {:?}",display_info);
        // 获取显示器显示区域
        self.rect = display_info.rect;
        // 创建 2D 图像资源
        self.resource_create_2d(
            RESOURCE_ID_FB,
            display_info.rect.width,
            display_info.rect.height,
        )?;

        // 分配资源图像的内存，后续显示器数据就直接操作该内存地址
        let size = display_info.rect.width * display_info.rect.height * 4;
        let frame_buffer_dma = DMA::new(pages(size as usize))?;

        // 设置图像资源的内存操作地址
        self.resource_attach_backing(RESOURCE_ID_FB, frame_buffer_dma.p_addr() as u64, size)?;

        // 将图像资源输出到显示器0
        self.set_scanout(display_info.rect, SCANOUT_ID, RESOURCE_ID_FB)?;

        // 记录地址
        let buf = unsafe { frame_buffer_dma.as_buf() };
        self.frame_buffer_dma = Some(frame_buffer_dma);
        Ok(buf)
    }

    ///
    /// 将数据刷入 GPU
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/13
    pub fn flush(&mut self) -> Result {
        self.transfer_to_host_2d(self.rect, 0, RESOURCE_ID_FB)?;
        self.resource_flush(self.rect, RESOURCE_ID_FB)?;
        Ok(())
    }

    ///
    /// 设置光标
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/13
    pub fn setup_cursor(&mut self, cursor_image: &[u8], pos_x: u32, pos_y: u32, hot_x: u32, hot_y: u32) -> Result {
        // 计算光标内存大小
        let size = CURSOR_RECT.width * CURSOR_RECT.height * 4;
        if cursor_image.len() != size as usize {
            return Err(Error::InvalidParam);
        }

        // 创建光标操作内存
        let cursor_buffer_dma = DMA::new(pages(size as usize))?;
        let buf = unsafe { cursor_buffer_dma.as_buf() };
        buf.copy_from_slice(cursor_image);

        // 为光标创建一个 2D 图层
        self.resource_create_2d(RESOURCE_ID_CURSOR, CURSOR_RECT.width, CURSOR_RECT.height)?;
        // 设置光标内存操作地址
        self.resource_attach_backing(RESOURCE_ID_CURSOR, cursor_buffer_dma.p_addr() as u64, size)?;
        // 将数据同步到 GPU
        self.transfer_to_host_2d(CURSOR_RECT, 0, RESOURCE_ID_CURSOR)?;
        // 更新光标位置
        self.update_cursor(RESOURCE_ID_CURSOR, SCANOUT_ID, pos_x, pos_y, hot_x, hot_y, false)?;
        // 记录光标操作内存地址
        self.cursor_buffer_dma = Some(cursor_buffer_dma);
        Ok(())
    }

    ///
    /// 移动光标
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/13
    pub fn move_cursor(&mut self, pos_x: u32, pos_y: u32) -> Result {
        self.update_cursor(RESOURCE_ID_CURSOR, SCANOUT_ID, pos_x, pos_y, 0, 0, true)?;
        Ok(())
    }

    ///
    /// 向 GPU 发送请求
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn request<Req, Rsp>(&mut self, req: Req) -> Result<Rsp> {
        // 写入请求信息
        unsafe {
            (self.queue_buf_send.as_mut_ptr() as *mut Req).write(req);
        }
        // 发送并接受请求
        self.control_queue.add(&[self.queue_buf_send], &[self.queue_buf_recv])?;
        self.header.notify(QUEUE_TRANSMIT as u32);
        while !self.control_queue.can_pop() {
            spin_loop();
        }
        // 回收 desc
        self.control_queue.pop_used()?;
        // 读取返回数据
        Ok(unsafe { (self.queue_buf_recv.as_ptr() as *const Rsp).read() })
    }

    ///
    /// 光标请求
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn cursor_request<Req>(&mut self, req: Req) -> Result {
        unsafe {
            (self.queue_buf_send.as_mut_ptr() as *mut Req).write(req);
        }
        self.cursor_queue.add(&[self.queue_buf_send], &[])?;
        self.header.notify(QUEUE_CURSOR as u32);
        while !self.cursor_queue.can_pop() {
            spin_loop();
        }
        self.cursor_queue.pop_used()?;
        Ok(())
    }

    ///
    /// 获取显示器详情
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn get_display_info(&mut self) -> Result<RespDisplayInfo> {
        let info: RespDisplayInfo = self.request(CtrlHeader::with_type(Command::GetDisplayInfo))?;
        info.header.check_type(Command::OkDisplayInfo)?;
        Ok(info)
    }

    ///
    /// 创建 2d 图形界面
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn resource_create_2d(&mut self, resource_id: u32, width: u32, height: u32) -> Result {
        let rsp: CtrlHeader = self.request(StResourceCreate2D {
            header: CtrlHeader::with_type(Command::ResourceCreate2d),
            resource_id,
            // 每个像素存储方式
            // Byte0 = Blue
            // Byte1 = Green
            // Byte2 = Red
            // Byte3 = Alpha（透明度）
            format: Format::B8G8R8A8UNORM,
            width,
            height,
        })?;
        rsp.check_type(Command::OkNoData)
    }

    ///
    /// 设置图像输出显示器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/16
    fn set_scanout(&mut self, rect: Rect, scanout_id: u32, resource_id: u32) -> Result {
        let rsp: CtrlHeader = self.request(StSetScanout {
            header: CtrlHeader::with_type(Command::SetScanout),
            rect,
            scanout_id,
            resource_id,
        })?;
        rsp.check_type(Command::OkNoData)
    }

    ///
    /// 将数据刷入 GPU
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn resource_flush(&mut self, rect: Rect, resource_id: u32) -> Result {
        let rsp: CtrlHeader = self.request(StResourceFlush {
            header: CtrlHeader::with_type(Command::ResourceFlush),
            rect,
            resource_id,
            _padding: 0,
        })?;
        rsp.check_type(Command::OkNoData)
    }

    ///
    /// 刷新显示器
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn transfer_to_host_2d(&mut self, rect: Rect, offset: u64, resource_id: u32) -> Result {
        let rsp: CtrlHeader = self.request(StTransferToHost2D {
            header: CtrlHeader::with_type(Command::TransferToHost2d),
            rect,
            offset,
            resource_id,
            _padding: 0,
        })?;
        rsp.check_type(Command::OkNoData)
    }

    ///
    /// 设置图像资源的内存地址
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn resource_attach_backing(&mut self, resource_id: u32, p_addr: u64, length: u32) -> Result {
        let rsp: CtrlHeader = self.request(StResourceAttachBacking {
            header: CtrlHeader::with_type(Command::ResourceAttachBacking),
            resource_id,
            nr_entries: 1,
            addr: p_addr,
            length,
            _padding: 0,
        })?;
        rsp.check_type(Command::OkNoData)
    }

    ///
    /// 更新光标位置
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/15
    fn update_cursor(&mut self, resource_id: u32, scanout_id: u32, pos_x: u32, pos_y: u32, hot_x: u32, hot_y: u32, is_move: bool) -> Result {
        self.cursor_request(StUpdateCursor {
            header: if is_move {
                CtrlHeader::with_type(Command::MoveCursor)
            } else {
                CtrlHeader::with_type(Command::UpdateCursor)
            },
            pos: CursorPos {
                scanout_id,
                x: pos_x,
                y: pos_y,
                _padding: 0,
            },
            resource_id,
            hot_x,
            hot_y,
            _padding: 0,
        })
    }
}
