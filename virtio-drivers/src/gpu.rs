use crate::hal::DMA;
use crate::queue::VirtQueue;
use crate::{pages, Error, Hal, VirtIOHeader};
use crate::{Result, PAGE_SIZE};
use bitflags::bitflags;
use core::fmt;
use core::fmt::Formatter;
use core::hint::spin_loop;
use core::ptr::dangling;
use log::info;
use volatile::{ReadOnly, Volatile, WriteOnly};

const QUEUE_TRANSMIT: usize = 0;
const QUEUE_CURSOR: usize = 1;

const SCANOUT_ID: u32 = 0;

const RESOURCE_ID_FB: u32 = 0xBABE;
const RESOURCE_ID_CURSOR: u32 = 0xDADE;

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

#[repr(u32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Command {
    GetDisplayInfo = 0x100,
    ResourceCreate2d = 0x101,
    ResourceUnRef = 0x102,
    SetScanout = 0x103,
    ResourceFlush = 0x104,
    TransferToHost2d = 0x105,
    ResourceAttachBacking = 0x106,
    ResourceDetachBacking = 0x107,
    GetCapSetInfo = 0x108,
    GetCapSet = 0x109,
    GetEdId = 0x10A,

    UpdateCursor = 0x300,
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

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CtrlHeader {
    hdr_type: Command,
    flags: u32,
    fence_id: u64,
    ctx_id: u32,
    _padding: u32,
}

impl CtrlHeader {
    fn with_type(hdr_type: Command) -> CtrlHeader {
        CtrlHeader {
            hdr_type,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            _padding: 0,
        }
    }

    fn check_type(&self, expected: Command) -> Result {
        if self.hdr_type == expected {
            Ok(())
        } else {
            Err(Error::IoError)
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct RespDisplayInfo {
    header: CtrlHeader,
    rect: Rect,
    enabled: u32,
    flags: u32,
}

#[repr(u32)]
#[derive(Debug)]
enum Format {
    B8G8R8A8UNORM = 1,
}

#[repr(C)]
#[derive(Debug)]
struct StResourceCreate2D {
    header: CtrlHeader,
    resource_id: u32,
    format: Format,
    width: u32,
    height: u32,
}

#[repr(C)]
#[derive(Debug)]
struct StResourceAttachBacking {
    header: CtrlHeader,
    resource_id: u32,
    nr_entries: u32, // 总是1
    addr: u64,
    length: u32,
    _padding: u32,
}

#[repr(C)]
#[derive(Debug)]
struct StSetScanout {
    header: CtrlHeader,
    rect: Rect,
    scanout_id: u32,
    resource_id: u32,
}

#[repr(C)]
#[derive(Debug)]
struct StTransferToHost2D {
    header: CtrlHeader,
    rect: Rect,
    offset: u64,
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

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct CursorPos {
    scanout_id: u32,
    x: u32,
    y: u32,
    _padding: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct StUpdateCursor {
    header: CtrlHeader,
    pos: CursorPos,
    resource_id: u32,
    hot_x: u32,
    hot_y: u32,
    _padding: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

///
/// 虚拟IO GPU
///
/// @author: tryte
///
/// @date: 2026/6/13
pub struct VirtIOGpu<'a, H: Hal> {
    header: &'static mut VirtIOHeader,
    rect: Rect,
    frame_buffer_dma: Option<DMA<H>>,
    cursor_buffer_dma: Option<DMA<H>>,
    control_queue: VirtQueue<'a, H>,
    cursor_queue: VirtQueue<'a, H>,
    queue_buf_dma: DMA<H>,
    queue_buf_send: &'a mut [u8],
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
        header.begin_init(|features| {
            let features = Features::from_bits_truncate(features);
            info!("device features: {:?}", features);
            let supported_features = Features::empty();
            (features & supported_features).bits()
        });

        let config = unsafe { &mut *(header.config_space() as *mut Config) };
        info!("config: {:?}",config);

        let control_queue = VirtQueue::new(header, QUEUE_TRANSMIT, 2)?;
        let cursor_queue = VirtQueue::new(header, QUEUE_CURSOR, 2)?;

        let queue_buf_dma = DMA::new(2)?;
        let queue_buf_send = unsafe { &mut queue_buf_dma.as_buf()[..PAGE_SIZE] };
        let queue_buf_recv = unsafe { &mut queue_buf_dma.as_buf()[PAGE_SIZE..] };

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
    /// 设置页缓存
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/13
    pub fn setup_framebuffer(&mut self) -> Result<&mut [u8]> {
        let display_info = self.get_display_info()?;
        info!("=> {:?}",display_info);
        self.rect = display_info.rect;

        self.resource_create_2d(
            RESOURCE_ID_FB,
            display_info.rect.width,
            display_info.rect.height,
        )?;

        let size = display_info.rect.width * display_info.rect.height * 4;
        let frame_buffer_dma = DMA::new(pages(size as usize))?;

        self.resource_attach_backing(RESOURCE_ID_FB, frame_buffer_dma.p_addr() as u64, size)?;

        self.set_scanout(display_info.rect, SCANOUT_ID, RESOURCE_ID_FB)?;

        let buf = unsafe { frame_buffer_dma.as_buf() };
        self.frame_buffer_dma = Some(frame_buffer_dma);
        Ok(buf)
    }

    ///
    /// 刷出内容
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
        let size = CURSOR_RECT.width * CURSOR_RECT.height * 4;
        if cursor_image.len() != size as usize {
            return Err(Error::InvalidParam);
        }

        let cursor_buffer_dma = DMA::new(pages(size as usize))?;
        let buf = unsafe { cursor_buffer_dma.as_buf() };
        buf.copy_from_slice(cursor_image);

        self.resource_create_2d(RESOURCE_ID_CURSOR, CURSOR_RECT.width, CURSOR_RECT.height)?;
        self.resource_attach_backing(RESOURCE_ID_CURSOR, cursor_buffer_dma.p_addr() as u64, size)?;
        self.transfer_to_host_2d(CURSOR_RECT, 0, RESOURCE_ID_CURSOR)?;
        self.update_cursor(RESOURCE_ID_CURSOR, SCANOUT_ID, pos_x, pos_y, hot_x, hot_y, false)?;
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

    fn request<Req, Rsp>(&mut self, req: Req) -> Result<Rsp> {
        unsafe {
            (self.queue_buf_send.as_mut_ptr() as *mut Req).write(req);
        }
        self.control_queue.add(&[self.queue_buf_send], &[self.queue_buf_recv])?;
        self.header.notify(QUEUE_TRANSMIT as u32);
        while !self.control_queue.can_pop() {
            spin_loop();
        }
        self.control_queue.pop_used()?;
        Ok(unsafe { (self.queue_buf_recv.as_ptr() as *const Rsp).read() })
    }

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

    fn get_display_info(&mut self) -> Result<RespDisplayInfo> {
        let info: RespDisplayInfo = self.request(CtrlHeader::with_type(Command::GetDisplayInfo))?;
        info.header.check_type(Command::OkDisplayInfo)?;
        Ok(info)
    }

    fn resource_create_2d(&mut self, resource_id: u32, width: u32, height: u32) -> Result {
        let rsp: CtrlHeader = self.request(StResourceCreate2D {
            header: CtrlHeader::with_type(Command::ResourceCreate2d),
            resource_id,
            format: Format::B8G8R8A8UNORM,
            width,
            height,
        })?;
        rsp.check_type(Command::OkNoData)
    }

    fn set_scanout(&mut self, rect: Rect, scanout_id: u32, resource_id: u32) -> Result {
        let rsp: CtrlHeader = self.request(StSetScanout {
            header: CtrlHeader::with_type(Command::SetScanout),
            rect,
            scanout_id,
            resource_id,
        })?;
        rsp.check_type(Command::OkNoData)
    }

    fn resource_flush(&mut self, rect: Rect, resource_id: u32) -> Result {
        let rsp: CtrlHeader = self.request(StResourceFlush {
            header: CtrlHeader::with_type(Command::ResourceFlush),
            rect,
            resource_id,
            _padding: 0,
        })?;
        rsp.check_type(Command::OkNoData)
    }

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
