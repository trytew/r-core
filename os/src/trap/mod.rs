mod context;

use core::arch::global_asm;
use riscv::register::mtvec::TrapMode;
use riscv::register::scause;
use riscv::register::stval;
use riscv::register::stvec;
use riscv::register::scause::Exception;
use riscv::register::scause::Trap;
pub use context::TrapContext;
use crate::batch::run_next_app;
use crate::println;
use crate::syscall::sys_call;

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
            run_next_app();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            run_next_app();
        }
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