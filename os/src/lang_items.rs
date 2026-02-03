use crate::println;
use crate::sbi::shutdown;
use core::panic::PanicInfo;

///
/// 处理 panic
///
/// @author: tryte
///
/// @date: 2025/11/20
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    if let Some(location) = _info.location() {
        println!(
            "Panicked at: {}:{} {}",
            location.file(),
            location.line(),
            _info.message(),
        );
    } else {
        println!("Panicked: {}", _info.message());
    }
    shutdown(true)
}
