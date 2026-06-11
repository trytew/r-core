use crate::syscall::sys_event_get;
use virtio_input_decoder::{DecodeType, Decoder};

#[repr(C)]
pub struct InputEvent {
    pub event_type: u16,
    pub code: u16,
    pub value: u32,
}

impl InputEvent {
    pub fn decode(&self) -> Option<DecodeType> {
        Decoder::decode(
            self.event_type as usize,
            self.code as usize,
            self.value as usize,
        )
        .ok()
    }
}

impl From<u64> for InputEvent {
    fn from(mut v: u64) -> Self {
        let value = v as u32;
        v >>= 32;
        let code = v as u16;
        v >>= 16;
        let event_type = v as u16;
        Self {
            event_type,
            code,
            value,
        }
    }
}

pub fn event_get() -> Option<InputEvent> {
    let raw_value = sys_event_get();
    if raw_value == 0 {
        None
    } else {
        Some((raw_value as u64).into())
    }
}
