#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![allow(rustdoc::macro_invocations)]

extern crate alloc;

mod boards;
mod config;
mod console;
mod lang_items;
mod loader;
mod mm;
mod sbi;
mod sync;
mod syscall;
mod task;
mod timer;
mod trap;

use core::arch::global_asm;

// 加载入口汇编文件
global_asm!(include_str!("./entry.asm"));
// 加载应用程序，该汇编代码由 build.rs 生成
global_asm!(include_str!("./linker_app.asm"));

///
/// 清空栈数据
///
/// @author: tryte
///
/// @date: 2025/11/17
fn clear_bss() {
    unsafe extern "C" {
        // sbss()并不是一个C库的函数，而是链接器脚本里定义的符号，只是被“当成函数指针”用来取得地址。
        // 为什么用 “fn sbss()” 而不是 “static sbss: u8”？因为 Rust 的 FFI 语法限制：
        //     1. 你不能在 extern "C" 块里定义一个“外部的地址符号”
        //     2. 但你 可以 定义“外部函数”，然后把它的地址当作符号地址读取
        fn sbss();
        fn ebss();
    }

    // (sbss as *const () as usize..ebss as *const () as usize)
    //     .for_each(|a| unsafe { (a as *mut u8).write_volatile(0) })

    // 将 bss 段数据置空，与上面代码效果一致
    unsafe {
        core::slice::from_raw_parts_mut(
            sbss as *mut u8,
            ebss as *const () as usize - sbss as *const () as usize,
        )
        .fill(0);
    }

    // 上面和以下的两种方式只能全局使用一种，如果 fn sbss() 和 static mut sbss 混用则会报符号命名重复
    // unsafe extern "C" {
    //     static mut sbss: u8;
    //     static mut ebss: u8;
    // }
    //
    // unsafe {
    //     let start = &raw mut sbss as *mut u8;
    //     let end = &raw mut ebss as *mut u8;
    //     let mut p = start;
    //     while p < end {
    //         p.write_volatile(0);
    //         p = p.add(1);
    //     }
    // }
}

#[unsafe(no_mangle)]
fn rust_main() -> ! {
    clear_bss();

    mm::init();
    mm::remap_test();

    trap::init();
    trap::enable_timer_interrupt();

    timer::set_next_tigger();

    task::run_first_task();

    println!("Hello world!");
    panic!("Shutdown machine!");
}
