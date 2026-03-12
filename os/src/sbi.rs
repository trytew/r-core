///
/// 从终端打印字符
///
/// @author: tryte
///
/// @date: 2025/11/17
pub fn console_put_char(c: usize) {
    #[allow(deprecated)]
    sbi_rt::legacy::console_putchar(c);
}

///
/// 从终端读取字符
///
/// @author: tryte
///
/// @date: 2026/3/12
pub fn console_get_char() -> usize {
    #[allow(deprecated)]
    sbi_rt::legacy::console_getchar()
}

///
/// 关机
///
/// @author: tryte
///
/// @date: 2025/11/18
pub fn shutdown(failure: bool) -> ! {
    use sbi_rt::system_reset;
    use sbi_rt::{NoReason, Shutdown, SystemFailure};

    if !failure {
        system_reset(Shutdown, NoReason);
    } else {
        system_reset(Shutdown, SystemFailure);
    }
    unreachable!()
}

///
/// 设置定时中断
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn set_timer(timer: usize) {
    sbi_rt::set_timer(timer as _);
}
