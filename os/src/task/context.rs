use crate::trap::trap_return;

///
/// 应用上下文
///
/// @author: tryte
///
/// @date: 2025/12/17
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskContext {
    ra: usize,      // 记录任务恢复后需要执行的下一条指令地址
    sp: usize,      // 应用栈栈顶指针
    s: [usize; 12], // s0~s11 寄存器的值
}

impl TaskContext {
    ///
    /// 初始化
    ///
    /// @author: tryte
    ///
    /// @date: 2025/12/17
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    ///
    /// 初始化应用上下文
    ///
    /// @author: tryte
    ///
    /// @date: 2026/1/31
    pub fn goto_trap_return(k_stack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: k_stack_ptr,
            s: [0; 12],
        }
    }
}
