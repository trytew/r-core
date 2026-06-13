use crate::hal::DMA;
use crate::queue::VirtQueue;
use crate::{Error, Hal, VirtIOHeader};
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

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

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
        let queue_buf_send = unsafe { &mut queue_buf_dma.as_buff()[..PAGE_SIZE] };
        let queue_buf_recv = unsafe { &mut queue_buf_dma.as_buff()[PAGE_SIZE..] };

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

    pub fn ack_interrupt(&mut self) -> bool {
        self.header.ack_interrupt()
    }

    pub fn resolution(&self) -> (u32, u32) {
        (self.rect.width, self.rect.height)
    }

    pub fn setup_framebuffer(&mut self) -> Result<&mut [u8]> {
        let display_info = self.get_display_info()?;
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

    fn get_display_info(&mut self) -> Result<RespDisplayInfo> {
        let info: RespDisplayInfo = self.request(CtrlHeader::with_type(Command::GetDisplayInfo))?;
        info.header.check_type(Command::OkDisplayInfo)?;
        Ok(info)
    }
}
