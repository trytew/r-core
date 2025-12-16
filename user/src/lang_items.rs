// #![feature(panic_info_message)]
use crate::println;
use core::panic::PanicInfo;

///
/// 处理 panic
///
/// @author: tryte
///
/// @date: 2025/11/20
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    if let Some(location) = panic_info.location() {
        println!(
            "Panicked at: {}:{} {}",
            location.file(),
            location.line(),
            panic_info.message(),
        );
    } else {
        println!("Panicked: {}", panic_info.message());
    }
    loop {}
}
