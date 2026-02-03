use crate::boards::CLOCK_FREQ;
use crate::sbi::set_timer;
use riscv::register::time;

const TICKS_PER_SEC: usize = 10;
const MSEC_PER_SEC: usize = 1000;

///
/// 获取时间
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn get_time() -> usize {
    time::read()
}

///
/// 获取毫秒
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

///
/// 设置下一个定时中断
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn set_next_tigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}
