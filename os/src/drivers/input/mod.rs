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

///
/// 输入驱动
///
/// @author: tryte
///
/// @date: 2026/6/10
struct VirtIOInputInner {
    /// 驱动封装
    virtio_input: VirtIOInput<'static, VirtioHal>,
    /// 事件
    events: VecDeque<u64>,
}

///
/// 输入驱动包装
///
/// @author: tryte
///
/// @date: 2026/6/10
struct VirtIOInputWrapper {
    /// 输入驱动
    inner: UpIntrFreeCell<VirtIOInputInner>,
    /// 条件变量
    cond_var: CondVar,
}

impl VirtIOInputWrapper {
    ///
    /// 创建虚拟IO驱动
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/10
    pub fn new(addr: usize) -> Self {
        // 创建驱动
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
    ///
    /// 读取驱动事件
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/10
    fn read_event(&self) -> u64 {
        loop {
            let mut inner = self.inner.exclusive_access();
            // 弹出事件
            if let Some(event) = inner.events.pop_front() {
                return event;
            } else {
                // 没有事件，阻塞等待
                let task_cx_ptr = self.cond_var.wait_no_sched();
                drop(inner);
                schedule(task_cx_ptr);
            }
        }
    }

    ///
    /// 中断处理
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/10
    fn handle_irq(&self) {
        let mut count = 0;
        let mut result = 0;
        self.inner.exclusive_session(|inner| {
            // 应答事件
            inner.virtio_input.ack_interrupt();
            // 读取事件
            while let Some(event) = inner.virtio_input.pop_pending_event() {
                // 记录事件数量
                count += 1;
                // 记录结果
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
