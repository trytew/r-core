use riscv::register::sstatus;
use riscv::register::sstatus::{Sstatus, SPP};

///
/// Trap 上下文
///
/// “用户态 → 内核态”这一瞬间，CPU 的现场
/// 把用户程序的全部寄存器状态保存下来，以后还能原样回去，
/// 因此以下字段存的都是用户态的内容
///
/// > 内存是C语言的内存布局
///
/// @author: tryte
///
/// @date: 2025/12/11
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrapContext {
    pub x: [usize; 32],      // 寄存器，32个
    pub sstatus: Sstatus,    // CSR 状态
    pub sepc: usize,         // CSR spec，这里记录的是 sret 之后需要执行的指令地址，非常重要
    pub kernel_satp: usize,  // 内核页表地址
    pub kernel_sp: usize,    // 内核栈顶，注意：该虚拟地址为内核虚拟地址页表的地址
    pub trap_handler: usize, // trap_handler 处理函数地址，注意：该虚拟地址为内核虚拟地址页表的地址
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
    /// 初始化应用“陷入”上下文
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/9
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,  //内核页表地址
        kernel_sp: usize,    // 内核栈顶
        trap_handler: usize, // “陷入”处理函数地址
    ) -> Self {
        // 读取 CSR 状态
        let mut sstatus = sstatus::read();
        // 设置特权级为用户级，该设置不是立马生效，而是等 sret 指令执行返回后生效
        sstatus.set_spp(SPP::User);
        // 记录用户态时寄存器的状态
        // 栈内存分布
        //    high addr
        // |             | 栈底
        // |     8kb     |
        // |-------------| --> sp 栈顶
        // |             |
        // |             |
        // |             |
        // |             | boot_stack_lower_bound 栈的下限位置
        //    lower addr
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // 记录应用的起始执行地址，在 __restore 执行后生效
            kernel_satp,
            kernel_sp,
            trap_handler,
        };
        // 记录用户栈栈顶
        cx.set_sp(sp);
        cx
    }
}
