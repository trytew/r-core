mod context;

use crate::config::TRAMPOLINE;
use crate::println;
use crate::syscall::sys_call;
use crate::task::{
    check_signals_error_of_current, current_add_signal, current_trap_cx, current_trap_cx_user_va,
    suspend_current_and_run_next, SignalFlags,
};
use crate::task::{current_user_token, exit_current_and_run_next};
use crate::timer::{check_timer, set_next_tigger};
pub use context::TrapContext;
use core::arch::{asm, global_asm};
use riscv::register::mtvec::TrapMode;
use riscv::register::scause::{Exception, Interrupt, Trap};
use riscv::register::{scause, sie, stval, stvec};

global_asm!(include_str!("./trap.asm"));

///
/// 初始化“陷入”处理
///
/// @author: tryte
///
/// @date: 2026/1/30
pub fn init() {
    set_kernel_trap_entry();
}

///
/// 设置内核态触发“陷入”的处理函数
///
/// @author: tryte
///
/// @date: 2026/1/30
fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as *const () as usize, TrapMode::Direct);
    }
}

///
/// 设置用户态触发“陷入”的处理函数
///
/// @author: tryte
///
/// @date: 2026/1/30
fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE, TrapMode::Direct);
    }
}

#[unsafe(no_mangle)]
fn trap_from_kernel() -> ! {
    panic!("a trap from kernel!");
}

///
/// 开启时间中断
///
/// @author: tryte
///
/// @date: 2026/1/4
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

///
/// “陷入”处理函数
///
///
/// @author: tryte
///
/// @date: 2026/1/29
#[unsafe(no_mangle)]
pub fn trap_handler() -> ! {
    // 设置内核态触发“陷入”时的处理函数
    set_kernel_trap_entry();
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            // 移动到 ecall 之后的指令，在 RISC-V 中 ecall 指令定长 4 字节
            // 获取应用“陷入”上下文物理地址
            let mut cx = current_trap_cx();
            cx.sepc += 4;
            let result = sys_call(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
            // 如果触发的系统调用是 sys_yield，那么进程将会发生切换，因此需要重新获取“陷入”上下文并设置系统调用结果
            cx = current_trap_cx();
            cx.x[10] = result;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            // println!(
            //     "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.\n",
            //     scause.cause(),
            //     stval,
            //     current_trap_cx().sepc,
            // );
            // exit_current_and_run_next(-2);
            current_add_signal(SignalFlags::SIGSEGV);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            // println!("[kernel] IllegalInstruction in application, kernel killed it.\n");
            // exit_current_and_run_next(-3);
            current_add_signal(SignalFlags::SIGILL);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_tigger();
            check_timer();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval,
            )
        }
    }

    // 检查当前进程有没有错误信号，有的话就退出当前进程
    if let Some((errno, msg)) = check_signals_error_of_current() {
        // 获取当前进程的控制块
        println!("[kernel] {}", msg);
        exit_current_and_run_next(errno);
    }

    trap_return();
}

///
/// 处理用户态触发“陷入”后的返回
///
/// @author: tryte
///
/// @date: 2026/1/31
#[unsafe(no_mangle)]
pub fn trap_return() -> ! {
    // 设置应用用户态触发“陷入”时的处理函数地址
    set_user_trap_entry();
    // 获取当前应用的“陷入”上下文虚拟地址
    let trap_cx_user_va = current_trap_cx_user_va();
    // 获取用户空间的 MMU 设置
    let user_satp = current_user_token();
    unsafe extern "C" {
        fn __alltraps();
        fn __restore();
    }
    // 计算 __restore函数 在虚拟内存地址的位置
    let restore_va =
        __restore as *const () as usize - __alltraps as *const () as usize + TRAMPOLINE;
    unsafe {
        asm!(
            // 刷新指令缓存
            "fence.i",
            // 跳转到 __restore 函数入口，{restore_va} 是模板参数，紧接着后面的参数是用来替换这个模板的
            // 最终会变成
            // in(reg) 中的 reg = rust选举的寄存器，假设x1，restore_va = 0x00aa，即：
            // mv x1, 0x00aa
            // jr x1
            "jr {restore_va}", restore_va = in(reg) restore_va,
            // in(...) / out(...) 是进入 asm 前的寄存器状态要求，换而言之下面两句一定会比前面两句代码更早执行，
            // 在 rust 的 asm! 中，分为声明和指令模板，如 "fence.i"/"jr {restore_va}" 这种带双引号的就是指令模板，有顺序要求
            // 而其余的则是声明，而且同一个寄存器不能多次作为输入源，{x}的同名占位只能声明一次，不能重复声明，因此声明乱序不会影响汇编代码的执行结果
            // 将“陷入”上下文作为参数传入
            in("a0") trap_cx_user_va,
            // 将用户空间的 MMU 设置作为参数传入
            in("a1") user_satp,
            // 告知 rust 该函数没有返回
            options(noreturn),
        )
    }
}
