
///
/// 打印字符
///
/// @author: tryte
///
/// @date: 2025/11/17
pub fn console_put_char(c: usize) {
    #[allow(deprecated)]
    sbi_rt::legacy::console_putchar(c);
}

///
/// 关机
///
/// @author: tryte
///
/// @date: 2025/11/18
pub fn shutdown(failure: bool) -> ! {

    use sbi_rt::NoReason;
    use sbi_rt::Shutdown;
    use sbi_rt::SystemFailure;
    use sbi_rt::system_reset;

    if !failure {
        system_reset(Shutdown, NoReason);
    } else {
        system_reset(Shutdown, SystemFailure);
    }
    unreachable!()
}
