use crate::drivers::chardev::UART;
use crate::drivers::{KEYBOARD_DEVICE, MOUSE_DEVICE};

///
/// 获取中断事件
///
/// @author: tryte
///
/// @date: 2026/6/11
pub fn sys_event_get() -> isize {
    let kb = KEYBOARD_DEVICE.clone();
    let mouse = MOUSE_DEVICE.clone();

    if !kb.is_empty() {
        kb.read_event() as isize
    } else if !mouse.is_empty() {
        mouse.read_event() as isize
    } else {
        0
    }
}

pub fn sys_key_pressed() -> isize {
    let res = !UART.read_buffer_is_empty();
    if res { 1 } else { 0 }
}
