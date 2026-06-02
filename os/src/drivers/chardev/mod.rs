use alloc::sync::Arc;
use lazy_static::lazy_static;

mod ns16550a;

use crate::boards::CharDeviceImpl;
pub use ns16550a::*;

lazy_static! {
    // 实例化 UART 设备
    pub static ref UART: Arc<CharDeviceImpl> = Arc::new(CharDeviceImpl::new());
}

///
/// 字符设备接口
///
/// @author: tryte
///
/// @date: 2026/6/2
pub trait CharDevice {
    ///
    /// 初始化
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    fn init(&self);
    ///
    /// 读
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    fn read(&self) -> u8;
    ///
    /// 写
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    fn write(&self, ch: u8);
    ///
    /// 中断处理函数
    ///
    /// @author: tryte
    ///
    /// @date: 2026/6/2
    fn handle_irq(&self);
}
