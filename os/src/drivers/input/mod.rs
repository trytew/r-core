use crate::drivers::bus::virtio::VirtioHal;
use crate::sync::{CondVar, UpIntrFreeCell};
use crate::task::schedule;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::any::Any;
use lazy_static::lazy_static;
use virtio_drivers::{VirtIOHeader, VirtIOInput};

lazy_static! {
    /// 键盘输入
    pub static ref KEYBOARD_DEVICE: Arc<dyn InputDevice> = Arc::new(VirtIOInputWrapper::new(VIRTIO5));

    /// 鼠标输入
    pub static ref MOUSE_DEVICE: Arc<dyn InputDevice> = Arc::new(VirtIOInputWrapper::new(VIRTIO6));
}

/// 键盘外设
const VIRTIO5: usize = 0x10005000;

/// 鼠标外设
const VIRTIO6: usize = 0x10006000;

pub trait InputDevice: Send + Sync + Any {
    fn read_event(&self) -> u64;
    fn handle_irq(&self);
    fn is_empty(&self) -> bool;
}

struct VirtIOInputInner {
    virtio_input: VirtIOInput<'static, VirtioHal>,
    events: VecDeque<u64>,
}

struct VirtIOInputWrapper {
    inner: UpIntrFreeCell<VirtIOInputInner>,
    cond_var: CondVar,
}

impl VirtIOInputWrapper {
    pub fn new(addr: usize) -> Self {
        let inner = VirtIOInputInner {
            virtio_input: unsafe {
                // let vt = addr as *const u8;
                // println!("{:#x}", core::ptr::read_volatile(vt.add(0)));
                // println!("{:#x}", core::ptr::read_volatile(vt.add(1)));
                // println!("{:#x}", core::ptr::read_volatile(vt.add(2)));
                // println!("{:#x}", core::ptr::read_volatile(vt.add(3)));
                VirtIOInput::<VirtioHal>::new(&mut *(addr as *mut VirtIOHeader)).unwrap()
            },
            events: VecDeque::new(),
        };
        Self {
            inner: unsafe { UpIntrFreeCell::new(inner) },
            cond_var: CondVar::new(),
        }
    }
}

impl InputDevice for VirtIOInputWrapper {
    fn read_event(&self) -> u64 {
        loop {
            let mut inner = self.inner.exclusive_access();
            if let Some(event) = inner.events.pop_front() {
                return event;
            } else {
                let task_cx_ptr = self.cond_var.wait_no_sched();
                drop(inner);
                schedule(task_cx_ptr);
            }
        }
    }

    fn handle_irq(&self) {
        let mut count = 0;
        let mut result = 0;
        self.inner.exclusive_session(|inner| {
            inner.virtio_input.ack_interrupt();
            while let Some(event) = inner.virtio_input.pop_pending_event() {
                count += 1;
                result = (event.event_type as u64) << 48
                    | (event.code as u64) << 32
                    | (event.value) as u64;
                inner.events.push_back(result);
            }
        });
        if count > 0 {
            self.cond_var.signal();
        }
    }

    fn is_empty(&self) -> bool {
        self.inner.exclusive_access().events.is_empty()
    }
}
