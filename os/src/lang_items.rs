use crate::println;
use crate::sbi::shutdown;
use crate::task::current_kernel_stack_top;
use core::arch::asm;
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
    backtrace();
    shutdown(true)
}

fn backtrace() {
    let mut fp: usize;
    let stop = current_kernel_stack_top();
    unsafe {
        asm!("mv {}, s0",out(reg) fp);
    }
    println!("--- START BACKTRACE ---");
    for i in 0..10 {
        if fp == stop {
            break;
        }
        unsafe {
            println!("#{}:ra={:x}", i, *((fp - 8) as *const usize));
            fp = *((fp - 16) as *const usize);
        }
    }
    println!("--- END BACKTRACE ---");
}
