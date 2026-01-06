#![no_std]
#![no_main]

mod lang_items;
mod sbi;
mod console;
mod batch;
mod sync;
mod trap;
mod syscall;

use core::arch::global_asm;

// 加载入口汇编文件
global_asm!(include_str!("./entry.asm"));
// 加载应用程序，该汇编代码由 build.rs 生成
global_asm!(include_str!("./linker_app.asm"));

#[unsafe(no_mangle)]
fn rust_main() -> ! {
    unsafe extern "C" {
        fn stext(); // text 段起始位置
        fn etext(); // text 段结束位置
        fn srodata(); // 只读段起始位置
        fn erodata(); // 只读段结束位置
        fn sdata(); // 常量数据段起始位置
        fn edata(); // 常量数据段结束位置
        fn sbss(); // 全局静态变量数据段起始位置
        fn ebss(); // 全局静态变量数据段结束位置
        fn boot_stack_lower_bound(); // 栈下限位置（栈内存的最低地址）
        fn boot_stack_top(); // 栈顶（栈的当前已使用地址）
    }
    clear_bss();

    println!("[kernel] Hello, world!\n");
    // 初始化 trap 上下文
    trap::init();
    batch::init();
    batch::run_next_app();

    println!("Hello world!");
    panic!("Shutdown machine!");
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
        static mut sbss: u8;
        static mut ebss: u8;
    }

    unsafe {
        let start = &raw mut sbss as *mut u8;
        let end = &raw mut ebss as *mut u8;
        let mut p = start;
        while p < end {
            p.write_volatile(0);
            p = p.add(1);
        }
    }
}
