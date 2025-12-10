use riscv::register::sstatus;
use riscv::register::sstatus::{Sstatus, SPP};

#[repr(C)]
pub struct TrapContext {
    // 寄存器，32个
    pub x: [usize; 32],
    // CSR 状态
    pub sstatus: Sstatus,
    // CSR spec
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
        // 设置特权级为用户级
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
        };
        // 设置用户栈指针
        cx.set_sp(sp);
        cx
    }
}

