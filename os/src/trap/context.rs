use riscv::register::sstatus;
use riscv::register::sstatus::{SPP, Sstatus};

///
/// Trap 上下文
///
/// 因为内存布局使用 C
///
/// @author: tryte
///
/// @date: 2025/12/11
#[repr(C)]
pub struct TrapContext {
    // 寄存器，32个
    pub x: [usize; 32],
    // CSR 状态
    pub sstatus: Sstatus,
    // CSR spec，这里记录的是应用程序的执行地址，非常重要
    pub sepc: usize,
}

impl TrapContext {
    ///
    /// 设置栈指针到 sp（x2） 寄存器
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/9
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    ///
    /// 初始化应用上下文
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/9
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        // 设置特权级为用户级，该设置不是立马生效，而是等 sret 指令执行返回后生效
        sstatus.set_spp(SPP::User);
        // 记录用户态时寄存器的状态
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // 记录应用的起始执行地址，每次都会通过 trap.asm 重置指令执行寄存器的值
        };
        // 记录用户栈栈顶
        //    high addr
        // |             | 栈底
        // |     8kb     |
        // |-------------| --> sp 栈顶
        // |             |
        // |             |
        // |             |
        // |             | boot_stack_lower_bound 栈的下限位置
        //    lower addr
        cx.set_sp(sp);
        cx
    }
}
