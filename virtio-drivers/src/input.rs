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
        const NOTIFY_ON_EMPTY = 1 << 24;
        const ANY_LAYOUT = 1 << 27;
        const RING_INDIRECT_DESC = 1 << 28;
        const RING_EVENT_IDX = 1 << 29;
        const UNUSED = 1 << 30;
        const VERSION_1 = 1 << 32;

        const ACCESS_PLATFORM = 1 << 33;
        const RING_PACKED = 1 << 34;
        const IN_ORDER = 1 << 35;
        const ORDER_PLATFORM = 1 << 36;
        const SR_IOV = 1 << 37;
        const NOTIFICATION_DATA = 1 << 38;
    }
}

#[repr(C)]
struct Config {
    select: WriteOnly<u8>,
    sub_sel: WriteOnly<u8>,
    size: ReadOnly<u8>,
    _reversed: [ReadOnly<u8>; 5],
    data: ReadOnly<[u8; 128]>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct InputEvent {
    pub event_type: u16,
    pub code: u16,
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

pub struct VirtIOInput<'a, H: Hal> {
    header: &'static mut VirtIOHeader,
    event_queue: VirtQueue<'a, H>,
    status_queue: VirtQueue<'a, H>,
    event_buf: Box<[InputEvent; 32]>,
}

impl<'a, H: Hal> VirtIOInput<'a, H> {
    pub fn new(header: &'static mut VirtIOHeader) -> Result<Self> {
        let mut event_buf = Box::new([InputEvent::default(); QUEUE_SIZE]);
        header.begin_init(|features| {
            let features = Feature::from_bits_truncate(features);
            info!("Device features: {:?}", features);
            let supported_features = Feature::empty();
            (features & supported_features).bits()
        });

        let mut event_queue = VirtQueue::new(header, QUEUE_EVENT, QUEUE_SIZE as u16)?;
        let status_queue = VirtQueue::new(header, QUEUE_EVENT, QUEUE_STATUS as u16)?;
        for (i, event) in event_buf.iter_mut().enumerate() {
            let token = event_queue.add(&[], &[event.as_buf_mut()])?;
            assert_eq!(token, i as u16);
        }

        header.finish_init();

        Ok(VirtIOInput {
            header,
            event_queue,
            status_queue,
            event_buf,
        })
    }

    pub fn ack_interrupt(&mut self) -> bool {
        self.header.ack_interrupt()
    }

    pub fn pop_pending_event(&mut self) -> Option<(u16, InputEvent)> {
        if let Ok((token, _)) = self.event_queue.pop_used() {
            let event = &mut self.event_buf[token as usize];
            if self.event_queue.add(&[], &[event.as_buf_mut()]).is_ok() {
                return Some((token, *event));
            }
        }
        None
    }

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

    pub fn virt_queue_size(&self) -> u16 {
        QUEUE_SIZE as u16
    }
}
