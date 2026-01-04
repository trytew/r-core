mod context;

use crate::println;
use crate::syscall::sys_call;
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};
use crate::timer::set_next_tigger;
pub use context::TrapContext;
use core::arch::global_asm;
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
/// 加载用户态寄存器状态保存函数
///
/// @author: tryte
///
/// @date: 2025/12/10
pub fn init() {
    unsafe extern "C" {
        safe fn __alltraps();
    }

    unsafe {
        // 设置 Trap 处理入口地址
        stvec::write(__alltraps as *const () as usize, TrapMode::Direct);
    }
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

#[unsafe(no_mangle)]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = sys_call(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, kernel killed it.");
            exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            exit_current_and_run_next();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_tigger();
            suspend_current_and_run_next();
        },
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval,
            )
        }
    }
    cx
}