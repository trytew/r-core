#![no_std]
#![no_main]

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

extern crate alloc;

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

    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) })

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
    // 测试内存页的分配和回收
    mm::init_heap();
    mm::init_frame_allocator();
    mm::frame_allocator_test();

    unsafe extern "C" {
        fn stext(); // text 段起始位置
        fn etext(); // text 段结束位置
        fn srodata(); // 只读段起始位置
        fn erodata(); // 只读段结束位置
        fn sdata(); // 常量数据段起始位置
        fn edata(); // 常量数据段结束位置
        fn boot_stack_lower_bound(); // 栈下限位置（栈内存的最低地址）
        fn boot_stack_top(); // 栈顶（栈的当前已使用地址）
    }
    clear_bss();

    println!("[kernel] Hello, world!\n");
    // 初始化 trap 上下文
    trap::init();
    loader::load_apps();
    trap::enable_timer_interrupt();
    timer::set_next_tigger();
    task::run_first_task();

    println!("Hello world!");
    panic!("Shutdown machine!");
}
