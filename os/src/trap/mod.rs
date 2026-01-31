mod context;

use crate::config::{TRAMPOLINE, TRAP_CONTEXT};
use crate::println;
use crate::syscall::sys_call;
use crate::task::{current_trap_cx, suspend_current_and_run_next};
use crate::task::{current_user_token, exit_current_and_run_next};
use crate::timer::set_next_tigger;
pub use context::TrapContext;
use core::arch::{asm, global_asm};
use riscv::register::medeleg::set_user_env_call;
use riscv::register::mtvec::TrapMode;
use riscv::register::scause;
use riscv::register::scause::Exception;
use riscv::register::scause::Interrupt;
use riscv::register::scause::Trap;
use riscv::register::sie;
use riscv::register::stval;
use riscv::register::stvec;

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
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
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
    set_kernel_trap_entry();
    let cx = current_trap_cx();
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = sys_call(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            println!("[kernel] PageFault in application, kernel killed it.\n");
            exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.\n");
            exit_current_and_run_next();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_tigger();
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
    trap_return()
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
    // 获取当前应用的寄存器状态
    let trap_cx_ptr = TRAP_CONTEXT;
    // 获取用户空间的 MMU 设置
    let user_satp = current_user_token();
    unsafe extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        asm!(
            // 刷新指令缓存
            "fence.i",
            // 跳转到 __restore 函数入口，{restore_va} 是模板参数，紧接着后面的参数是用来替换这个模板的
            "jr {restore_va}", restore_va = in(reg) restore_va,
            // 将“陷入”上下文作为参数传入
            in("a0") trap_cx_ptr,
            // 将用户空间的 MMU 设置作为参数传入
            in("a1") user_satp,
            // 告知 rust 该函数没有返回
            options(noreturn),
        )
    }
}
