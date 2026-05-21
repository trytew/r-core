// #![feature(panic_info_message)]
use crate::{getpid, kill, println, SignalFlags};
use core::panic::PanicInfo;

///
/// 处理 panic
///
/// @author: tryte
///
/// @date: 2025/11/20
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    let err = panic_info.message();
    if let Some(location) = panic_info.location() {
        println!(
            "Panicked at: {}:{} {}",
            location.file(),
            location.line(),
            err,
        );
    } else {
        println!("Panicked: {}", err);
    }
    kill(getpid() as usize, SignalFlags::SIGABRT.bits());
    unreachable!();
}
