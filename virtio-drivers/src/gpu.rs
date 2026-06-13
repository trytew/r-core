use crate::hal::DMA;
use crate::queue::VirtQueue;
use crate::{Hal, VirtIOHeader};

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

impl <H:Hal> VirtIOGpu<'_,H> {
    
}
