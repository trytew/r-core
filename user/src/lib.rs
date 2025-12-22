#![no_std]
#![feature(linkage)]

pub mod console;
mod syscall;
mod lang_items;

use syscall::*;

// 使用 Rust 的宏将 _start 这段代码编译后的汇编代码中放在一个名为 .text.entry 的代码段中，
// 在 linker.ld 脚本中指定了 .text.entry 在最开始的位置
// 方便我们在后续链接的时候调整它的位置使得它能够作为用户库的入口
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    clear_bss();
    exit(main());
    panic!("unreachable after sys_exit!")
}

// 使用 Rust 的宏将其函数符号 main 标志为弱链接。
// 这样在最后链接的时候，虽然在 lib.rs 和 bin 目录下的某个应用程序都有 main 符号，但由于 lib.rs 中的 main 符号是弱链接，
// 链接器会使用 bin 目录下的应用主逻辑作为 main 。这里我们主要是进行某种程度上的保护，如果在 bin 目录下找不到任何 main ，
// 那么编译也能够通过，但会在运行时报错
#[linkage = "weak"]
#[unsafe(no_mangle)]
fn main() -> i32 {
    panic!("Cannot found main");
}

///
/// 清空栈数据
///
/// @author: tryte
///
/// @date: 2025/11/17
fn clear_bss() {
    // unsafe extern "C" {
    //     // sbss()并不是一个C库的函数，而是链接器脚本里定义的符号，只是被“当成函数指针”用来取得地址。
    //     // 为什么用 “fn sbss()” 而不是 “static sbss: u8”？因为 Rust 的 FFI 语法限制：
    //     //     1. 你不能在 extern "C" 块里定义一个“外部的地址符号”
    //     //     2. 但你 可以 定义“外部函数”，然后把它的地址当作符号地址读取
    //     fn sbss();
    //     fn ebss();
    // }
    //
    // (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) })

    unsafe extern "C" {
        static mut start_bss: u8;
        static mut end_bss: u8;
    }

    unsafe {
        let start = &raw mut start_bss as *mut u8;
        let end = &raw mut end_bss as *mut u8;
        let mut p = start;
        while p < end {
            p.write_volatile(0);
            p = p.add(1);
        }
    }
}

///
/// 写入
///
/// @author: tryte
///
/// @date: 2025/11/20
pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

///
/// 退出
///
/// @author: tryte
///
/// @date: 2025/11/20
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}
